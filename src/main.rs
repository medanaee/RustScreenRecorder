use ashpd::desktop::{
    PersistMode,
    screencast::{CursorMode, Screencast, SelectSourcesOptions, SourceType},
};
use axum::{
    Json, Router,
    extract::State,
    http::{StatusCode, Uri, header},
    response::IntoResponse,
    routing::{get, post},
};
use gstreamer::prelude::*;
use gstreamer_app;
use rust_embed::RustEmbed;
use serde::Deserialize;
use std::error::Error;
use std::os::fd::IntoRawFd;
use std::process::Command;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;

#[derive(Clone, Debug, PartialEq)]
enum Status {
    Recording,
    Paused,
}

enum RecCommand {
    Stop,
    Pause,
    Resume,
}

struct StateData {
    tx: tokio::sync::mpsc::Sender<RecCommand>,
    status: Status,
}

struct AppStateInner {
    rec_data: Option<StateData>,
    fd_provider: Option<tokio::sync::mpsc::Sender<tokio::sync::oneshot::Sender<i32>>>,
    capture_node_id: Option<u32>,
    preview_tx: tokio::sync::broadcast::Sender<bytes::Bytes>,
    preview_pipeline: Option<gstreamer::Pipeline>,
}

type AppState = Arc<Mutex<AppStateInner>>;

#[derive(Deserialize, Debug)]
struct RecordConfig {
    fps: u32,
    quality_bitrate: u32,
    resolution: String,
    encoder: String,
    record_mic: bool,
    record_system_audio: bool,
    mic_source: String,
    output_folder: String,
    show_cursor: bool,
}


#[derive(Deserialize, Debug)]
struct InitConfig {
    show_cursor: bool,
}

#[derive(RustEmbed)]
#[folder = "frontend/dist/"]
struct Assets;

async fn static_handler(uri: Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };

    match Assets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            ([(header::CONTENT_TYPE, mime.as_ref())], content.data).into_response()
        }
        None => {
            if let Some(index) = Assets::get("index.html") {
                let mime = mime_guess::from_path("index.html").first_or_octet_stream();
                ([(header::CONTENT_TYPE, mime.as_ref())], index.data).into_response()
            } else {
                (StatusCode::NOT_FOUND, "404 Not Found").into_response()
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    gstreamer::init()?;

    let (preview_tx, _) = tokio::sync::broadcast::channel(16);
    let state: AppState = Arc::new(Mutex::new(AppStateInner {
        rec_data: None,
        fd_provider: None,
        capture_node_id: None,
        preview_tx,
        preview_pipeline: None,
    }));

    let app = Router::new()
        .route("/api/init", post(init_capture))
        .route("/api/preview_stream", get(preview_stream))
        .route("/api/start", post(start_recording))
        .route("/api/stop", post(stop_recording))
        .route("/api/pause", post(pause_recording))
        .route("/api/resume", post(resume_recording))
        .route("/api/exit", post(exit_app))
        .route("/api/status", get(get_status))
        .route("/api/mics", get(get_microphones))
        .route("/api/choose_path", post(choose_path))
        .route("/api/default_path", get(get_default_path))
        .fallback(static_handler)
        .layer(CorsLayer::permissive())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3001").await?;
    println!("Backend server running locally on http://127.0.0.1:3001");

    if let Err(e) = Command::new("xdg-open")
        .arg("http://127.0.0.1:3001")
        .spawn()
    {
        eprintln!("Failed to open browser automatically: {}", e);
    }

    axum::serve(listener, app).await?;
    Ok(())
}

async fn get_default_path() -> impl IntoResponse {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let path = format!("{}/Videos/RScreenRec", home);
    let _ = std::fs::create_dir_all(&path);
    (StatusCode::OK, path)
}

async fn choose_path() -> impl IntoResponse {
    let folder = rfd::AsyncFileDialog::new()
        .set_title("Choose save folder")
        .pick_folder()
        .await;

    match folder {
        Some(handle) => (StatusCode::OK, handle.path().to_string_lossy().to_string()),
        None => (StatusCode::BAD_REQUEST, "".to_string()),
    }
}

async fn get_microphones() -> impl IntoResponse {
    let monitor = gstreamer::DeviceMonitor::new();
    monitor.add_filter(Some("Audio/Source"), None);
    let _ = monitor.start();

    tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

    let mut mic_list = Vec::new();

    mic_list.push(serde_json::json!({
        "id": "default",
        "name": "Default Audio Source"
    }));

    for device in monitor.devices() {
        let name = device.display_name().to_string();
        if let Some(props) = device.properties() {
            let id = props
                .get::<String>("device.name")
                .or_else(|_| props.get::<String>("pulse.device.name"))
                .or_else(|_| props.get::<String>("object.path"))
                .unwrap_or_else(|_| name.clone());

            mic_list.push(serde_json::json!({
                "id": id,
                "name": name
            }));
        }
    }

    monitor.stop();
    Json(mic_list)
}

async fn get_status(State(state): State<AppState>) -> impl IntoResponse {
    let guard = state.lock().await;
    if let Some(data) = &guard.rec_data {
        match data.status {
            Status::Recording => (StatusCode::OK, "recording"),
            Status::Paused => (StatusCode::OK, "paused"),
        }
    } else {
        (StatusCode::OK, "idle")
    }
}

use std::process::Stdio;
use tokio::process::Command as TokioCommand;

async fn init_capture(State(state): State<AppState>, Json(config): Json<InitConfig>) -> impl IntoResponse {
    {
        let mut guard = state.lock().await;
        if let Some(old_pipeline) = guard.preview_pipeline.take() {
            let _ = old_pipeline.set_state(gstreamer::State::Null);
        }
    }

    let proxy = Screencast::new().await.unwrap();
    let session = proxy.create_session(Default::default()).await.unwrap();
    
    let cursor_mode = if config.show_cursor {
        CursorMode::Embedded
    } else {
        CursorMode::Hidden
    };

    proxy
        .select_sources(&session, SelectSourcesOptions::default().set_cursor_mode(cursor_mode))
        .await
        .unwrap();
    let response = proxy
        .start(&session, None, Default::default())
        .await
        .unwrap()
        .response()
        .unwrap();
        
    let stream = response.streams().first().unwrap();
    let node_id = stream.pipe_wire_node_id();

    // Fixed type mismatch: initialized variables as i32 to match stream.size() types
    let mut prev_w: i32 = 1280;
    let mut prev_h: i32 = 720;

    if let Some(size) = stream.size() {
        let (w, h) = size; // w and h are i32
        if w > 1280 || h > 720 {
            let scale = f32::min(1280.0 / w as f32, 720.0 / h as f32);
            prev_w = (w as f32 * scale) as i32;
            prev_h = (h as f32 * scale) as i32;
        } else {
            prev_w = w;
            prev_h = h;
        }
    }
    
    // Ensure dimensions are even numbers using i32 operations
    prev_w = (prev_w / 2) * 2;
    prev_h = (prev_h / 2) * 2;

    let (req_tx, mut req_rx) = tokio::sync::mpsc::channel::<tokio::sync::oneshot::Sender<i32>>(10);

    tokio::spawn(async move {
        let proxy = proxy;
        let session = session;
        while let Some(reply_tx) = req_rx.recv().await {
            if let Ok(fd) = proxy
                .open_pipe_wire_remote(&session, Default::default())
                .await
            {
                let _ = reply_tx.send(fd.into_raw_fd());
            } else {
                break;
            }
        }
    });

    let (res_tx, res_rx) = tokio::sync::oneshot::channel();
    req_tx.send(res_tx).await.unwrap();
    let preview_fd = res_rx.await.unwrap();

    let pipeline_str = format!(
        "pipewiresrc fd={} path={} do-timestamp=true ! queue max-size-buffers=30 max-size-time=0 max-size-bytes=0 leaky=downstream ! videoconvert ! videoscale ! video/x-raw,width={},height={},format=RGB ! videorate ! video/x-raw,framerate=15/1 ! jpegenc quality=80 ! appsink name=preview_sink drop=true max-buffers=1",
        preview_fd, node_id, prev_w, prev_h
    );

    let pipeline = gstreamer::parse::launch(&pipeline_str)
        .unwrap()
        .dynamic_cast::<gstreamer::Pipeline>()
        .unwrap();
    let appsink = pipeline
        .by_name("preview_sink")
        .unwrap()
        .dynamic_cast::<gstreamer_app::AppSink>()
        .unwrap();

    let mut guard = state.lock().await;
    guard.fd_provider = Some(req_tx);
    guard.capture_node_id = Some(node_id);
    guard.preview_pipeline = Some(pipeline.clone());
    let tx = guard.preview_tx.clone();

    appsink.set_callbacks(
        gstreamer_app::AppSinkCallbacks::builder()
            .new_sample(move |sink| {
                if let Ok(sample) = sink.pull_sample() {
                    if let Some(buffer) = sample.buffer() {
                        if let Ok(map) = buffer.map_readable() {
                            let _ = tx.send(bytes::Bytes::copy_from_slice(map.as_slice()));
                        }
                    }
                }
                Ok(gstreamer::FlowSuccess::Ok)
            })
            .build(),
    );
    pipeline.set_state(gstreamer::State::Playing).unwrap();

    (StatusCode::OK, "Initialized")
}

async fn preview_stream(State(state): State<AppState>) -> impl IntoResponse {
    let rx = state.lock().await.preview_tx.subscribe();
    let stream = async_stream::stream! {
        let mut rx = rx;
        loop {
            if let Ok(frame) = rx.recv().await {
                let mut buf = Vec::new();
                buf.extend_from_slice(b"--frame\r\nContent-Type: image/jpeg\r\n\r\n");
                buf.extend_from_slice(&frame);
                buf.extend_from_slice(b"\r\n");
                yield Ok::<_, std::io::Error>(buf);
            }
        }
    };
    (
        [(
            header::CONTENT_TYPE,
            "multipart/x-mixed-replace; boundary=frame",
        )],
        axum::body::Body::from_stream(stream),
    )
}

fn get_default_sink_monitor() -> Option<String> {
    let output = std::process::Command::new("pactl")
        .arg("get-default-sink")
        .output()
        .ok()?;
    let sink_name = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if sink_name.is_empty() { None } else { Some(format!("{}.monitor", sink_name)) }
}

async fn start_recording(
    State(state): State<AppState>,
    Json(config): Json<RecordConfig>,
) -> impl IntoResponse {
    let mut state_guard = state.lock().await;
    if state_guard.rec_data.is_some() {
        return (StatusCode::BAD_REQUEST, "Already recording").into_response();
    }

    let fd_provider = state_guard.fd_provider.clone().expect("Call init first");
    let node_id = state_guard.capture_node_id.expect("Call init first");

    let (tx, mut rx) = tokio::sync::mpsc::channel::<RecCommand>(10);
    state_guard.rec_data = Some(StateData {
        tx,
        status: Status::Recording,
    });
    let state_clone = Arc::clone(&state);

    tokio::spawn(async move {
        let mut audio_sources = Vec::new();
           
        if config.record_mic {
            let mic_dev = if config.mic_source == "default" || config.mic_source.is_empty() {
                "pulsesrc".to_string()
            } else {
                format!("pulsesrc device=\"{}\"", config.mic_source)
            };
            audio_sources.push(mic_dev);
        }

        if config.record_system_audio {
            if let Some(monitor) = get_default_sink_monitor() {
                audio_sources.push(format!("pulsesrc device=\"{}\"", monitor));
            } else {
                audio_sources.push("pulsesrc".to_string());
            }
        }

        let audio_pipeline = if audio_sources.is_empty() {
            "".to_string() 
        } else if audio_sources.len() == 1 {
            format!("{} ! queue ! audioconvert ! audioresample ! audiorate ! avenc_aac ! aacparse ! mux.", audio_sources[0])
        } else {
            format!("audiomixer name=mix ! queue ! audioconvert ! audioresample ! audiorate ! avenc_aac ! aacparse ! mux. {} ! queue ! mix. {} ! queue ! mix.", audio_sources[0], audio_sources[1])
        };

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let final_output_path = format!("{}/recording_{}.mkv", config.output_folder, timestamp);

        let mut segment_files = Vec::new();
        let mut current_pipeline: Option<gstreamer::Pipeline> = None;
        let mut segment_idx = 0;

        let scale_caps = if config.resolution == "original" {
            "".to_string()
        } else {
            let parts: Vec<&str> = config.resolution.split('x').collect();
            if parts.len() == 2 {
                format!(",width={},height={}", parts[0], parts[1])
            } else {
                "".to_string()
            }
        };

        let (res_tx, res_rx) = tokio::sync::oneshot::channel();
        if fd_provider.send(res_tx).await.is_err() {
            return;
        }
        let rec_fd = res_rx.await.unwrap();

        let path = format!("/tmp/seg_{}_{}.mkv", std::process::id(), segment_idx);

        let pipeline_str = match config.encoder.as_str() {
            "nvenc" => format!(
                "matroskamux name=mux ! filesink location=\"{}\" async=false \
                 pipewiresrc fd={} path={} do-timestamp=true ! queue max-size-buffers=200 max-size-time=0 max-size-bytes=0 leaky=downstream ! videoconvert ! videoscale ! video/x-raw,format=NV12{} ! videorate ! video/x-raw,framerate={}/1 ! queue max-size-buffers=200 max-size-time=0 max-size-bytes=0 leaky=downstream ! nvh264enc preset=low-latency-hq zerolatency=true rc-mode=cbr bitrate={} gop-size={} ! h264parse config-interval=-1 ! mux. \
                 {}",
                path, rec_fd, node_id, scale_caps, config.fps, config.quality_bitrate, config.fps, audio_pipeline
            ),
            "vaapi" => format!(
                            "matroskamux name=mux ! filesink location=\"{}\" async=false \
                             pipewiresrc fd={} path={} do-timestamp=true ! queue max-size-buffers=200 max-size-time=0 max-size-bytes=0 leaky=downstream ! videoconvert ! videoscale ! video/x-raw,format=I420{} ! videorate ! video/x-raw,framerate={}/1 ! queue max-size-buffers=200 max-size-time=0 max-size-bytes=0 leaky=downstream ! vaapih264enc bitrate={} rate-control=cbr ! h264parse config-interval=-1 ! mux. \
                             {}",
                            path, rec_fd, node_id, scale_caps, config.fps, config.quality_bitrate, audio_pipeline
            ),
            _ => format!( // fallback to x264
                "matroskamux name=mux ! filesink location=\"{}\" async=false \
                 pipewiresrc fd={} path={} do-timestamp=true ! queue max-size-buffers=200 max-size-time=0 max-size-bytes=0 leaky=downstream ! videoconvert ! videoscale ! video/x-raw,format=I420{} ! videorate ! video/x-raw,framerate={}/1 ! queue max-size-buffers=200 max-size-time=0 max-size-bytes=0 leaky=downstream ! x264enc speed-preset=ultrafast threads=0 bitrate={} key-int-max={} bframes=0 ! h264parse config-interval=-1 ! mux. \
                 {}",
                path, rec_fd, node_id, scale_caps, config.fps, config.quality_bitrate, config.fps, audio_pipeline
            ),
        };

        let pipeline = gstreamer::parse::launch(&pipeline_str)
            .expect("Failed to create GStreamer pipeline")
            .dynamic_cast::<gstreamer::Pipeline>()
            .expect("Failed to cast to Pipeline");

        pipeline
            .set_state(gstreamer::State::Playing)
            .expect("Failed to start playing");

        segment_files.push(path);
        current_pipeline = Some(pipeline);

        while let Some(cmd) = rx.recv().await {
            match cmd {
                RecCommand::Stop => break,
                RecCommand::Pause => {
                    if let Some(pipeline) = current_pipeline.take() {
                        pipeline.send_event(gstreamer::event::Eos::new());
                        let bus = pipeline.bus().unwrap();
                        let _ = tokio::task::spawn_blocking(move || {
                            for msg in bus.iter_timed(gstreamer::ClockTime::NONE) {
                                use gstreamer::MessageView;
                                match msg.view() {
                                    MessageView::Eos(..) | MessageView::Error(_) => break,
                                    _ => (),
                                }
                            }
                            pipeline.set_state(gstreamer::State::Null).unwrap();
                        })
                        .await;
                    }
                }
                RecCommand::Resume => {
                    segment_idx += 1;

                    let (res_tx, res_rx) = tokio::sync::oneshot::channel();
                    if fd_provider.send(res_tx).await.is_err() {
                        break;
                    }
                    let rec_fd = res_rx.await.unwrap();

                    let path = format!("/tmp/seg_{}_{}.mkv", std::process::id(), segment_idx);

                    let pipeline_str = match config.encoder.as_str() {
                    "nvenc" => format!(
                        "matroskamux name=mux ! filesink location=\"{}\" async=false \
                        pipewiresrc fd={} path={} do-timestamp=true ! queue max-size-buffers=200 max-size-time=0 max-size-bytes=0 leaky=downstream ! videoconvert ! videoscale ! video/x-raw,format=NV12{} ! videorate ! video/x-raw,framerate={}/1 ! queue max-size-buffers=200 max-size-time=0 max-size-bytes=0 leaky=downstream ! nvh264enc preset=low-latency-hq zerolatency=true rc-mode=cbr bitrate={} gop-size={} ! h264parse config-interval=-1 ! mux. \
                        {}",
                        path, rec_fd, node_id, scale_caps, config.fps, config.quality_bitrate, config.fps, audio_pipeline
                    ),
                    "vaapi" => format!(
                            "matroskamux name=mux ! filesink location=\"{}\" async=false \
                             pipewiresrc fd={} path={} do-timestamp=true ! queue max-size-buffers=200 max-size-time=0 max-size-bytes=0 leaky=downstream ! videoconvert ! videoscale ! video/x-raw,format=I420{} ! videorate ! video/x-raw,framerate={}/1 ! queue max-size-buffers=200 max-size-time=0 max-size-bytes=0 leaky=downstream ! vaapih264enc bitrate={} rate-control=cbr ! h264parse config-interval=-1 ! mux. \
                             {}",
                            path, rec_fd, node_id, scale_caps, config.fps, config.quality_bitrate, audio_pipeline
                    ),
                    _ => format!( // fallback to x264
                        "matroskamux name=mux ! filesink location=\"{}\" async=false \
                        pipewiresrc fd={} path={} do-timestamp=true ! queue max-size-buffers=200 max-size-time=0 max-size-bytes=0 leaky=downstream ! videoconvert ! videoscale ! video/x-raw,format=I420{} ! videorate ! video/x-raw,framerate={}/1 ! queue max-size-buffers=200 max-size-time=0 max-size-bytes=0 leaky=downstream ! x264enc speed-preset=ultrafast threads=0 bitrate={} key-int-max={} bframes=0 ! h264parse config-interval=-1 ! mux. \
                        {}",
                        path, rec_fd, node_id, scale_caps, config.fps, config.quality_bitrate, config.fps, audio_pipeline
                    ),
        };

                    let pipeline = gstreamer::parse::launch(&pipeline_str)
                        .expect("Failed to create GStreamer pipeline")
                        .dynamic_cast::<gstreamer::Pipeline>()
                        .expect("Failed to cast to Pipeline");

                    pipeline
                        .set_state(gstreamer::State::Playing)
                        .expect("Failed to start playing");

                    segment_files.push(path);
                    current_pipeline = Some(pipeline);
                }
            }
        }

        if let Some(pipeline) = current_pipeline.take() {
            pipeline.send_event(gstreamer::event::Eos::new());
            let bus = pipeline.bus().unwrap();
            let _ = tokio::task::spawn_blocking(move || {
                for msg in bus.iter_timed(gstreamer::ClockTime::NONE) {
                    use gstreamer::MessageView;
                    match msg.view() {
                        MessageView::Eos(..) | MessageView::Error(_) => break,
                        _ => (),
                    }
                }
                pipeline.set_state(gstreamer::State::Null).unwrap();
            })
            .await;
        }

        let concat_list = format!("/tmp/list_{}.txt", std::process::id());
        let mut list_content = String::new();
        for file in &segment_files {
            if std::fs::metadata(file).is_ok() {
                list_content.push_str(&format!("file '{}'\n", file));
            }
        }
        std::fs::write(&concat_list, list_content).unwrap();

        let concat_status = TokioCommand::new("ffmpeg")
            .args([
                "-y",
                "-f",
                "concat",
                "-safe",
                "0",
                "-i",
                &concat_list,
                "-c",
                "copy",
                &final_output_path,
            ])
            .status()
            .await;

        match concat_status {
            Ok(status) if status.success() => {
                for file in &segment_files {
                    let _ = std::fs::remove_file(file);
                }
                let _ = std::fs::remove_file(concat_list);
            }
            _ => {
                eprintln!(
                    "FFmpeg concat failed! Segments and list file are kept in /tmp/ for debugging."
                );
            }
        }

        state_clone.lock().await.rec_data = None;
    });

    (StatusCode::OK, "Started").into_response()
}


async fn stop_recording(State(state): State<AppState>) -> impl IntoResponse {
    let mut state_guard = state.lock().await;
    if let Some(data) = state_guard.rec_data.take() {
        let _ = data.tx.send(RecCommand::Stop).await;
        (StatusCode::OK, "Stopped")
    } else {
        (StatusCode::BAD_REQUEST, "Not recording")
    }
}

async fn pause_recording(State(state): State<AppState>) -> impl IntoResponse {
    let mut state_guard = state.lock().await;
    if let Some(data) = state_guard.rec_data.as_mut() {
        let _ = data.tx.send(RecCommand::Pause).await;
        data.status = Status::Paused;
        (StatusCode::OK, "Paused")
    } else {
        (StatusCode::BAD_REQUEST, "Not recording")
    }
}

async fn resume_recording(State(state): State<AppState>) -> impl IntoResponse {
    let mut state_guard = state.lock().await;
    if let Some(data) = state_guard.rec_data.as_mut() {
        let _ = data.tx.send(RecCommand::Resume).await;
        data.status = Status::Recording;
        (StatusCode::OK, "Resumed")
    } else {
        (StatusCode::BAD_REQUEST, "Not recording")
    }
}

async fn exit_app(State(state): State<AppState>) -> impl IntoResponse {
    if let Some(data) = state.lock().await.rec_data.take() {
        let _ = data.tx.send(RecCommand::Stop).await;
    }

    tokio::spawn(async {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        std::process::exit(0);
    });

    (StatusCode::OK, "Exiting")
}