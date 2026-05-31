import { useState, useEffect } from "react";
import { Settings, Monitor, Mic, Video, Folder, FolderOpen, MousePointer2, Cpu } from "lucide-react";
import type { RecordConfig } from "../types";
import { CustomSelect } from "./CustomSelect";

interface SettingsViewProps {
  config: RecordConfig;
  setConfig: (config: RecordConfig) => void;
  status: string;
}

const Toggle = ({ checked, onChange, disabled }: { checked: boolean, onChange: (v: boolean) => void, disabled?: boolean }) => (
  <div 
    onClick={() => !disabled && onChange(!checked)} 
    className={`w-10 h-5 flex items-center rounded-full p-1 cursor-pointer transition-colors ${disabled ? 'opacity-50 pointer-events-none' : ''} ${checked ? 'bg-blue-600' : 'bg-[#3f3f46]'}`}
  >
    <div className={`bg-white w-3.5 h-3.5 rounded-full shadow-md transform transition-transform ${checked ? 'translate-x-5' : ''}`} />
  </div>
);

export function SettingsView({ config, setConfig, status }: SettingsViewProps) {
  const [micList, setMicList] = useState<{id: string, name: string}[]>([]);
  const isDisabled = status !== "idle";

  useEffect(() => {
    fetch("http://127.0.0.1:3001/api/mics")
      .then(res => res.json())
      .then(data => setMicList(data))
      .catch(err => console.error("Could not fetch mics", err));
  }, []);

  const updateConfig = (key: keyof RecordConfig, value: any) => setConfig({ ...config, [key]: value });

  const handleChoosePath = async () => {
    try {
      const res = await fetch("http://127.0.0.1:3001/api/choose_path", { method: "POST" });
      if (res.ok) {
        const path = await res.text();
        if (path) updateConfig("output_folder", path);
      }
    } catch (e) {
      console.error("Failed to choose path", e);
    }
  };

  const resolutionOptions = [
    { value: "original", label: "Native Resolution" },
    { value: "1920x1080", label: "1920 x 1080 (1080p)" },
    { value: "1280x720", label: "1280 x 720 (720p)" },
    { value: "854x480", label: "854 x 480 (480p)" }
  ];

  const fpsOptions = [
    { value: 15, label: "15 FPS" },
    { value: 20, label: "20 FPS" },
    { value: 22, label: "22 FPS" },
    { value: 24, label: "24 FPS" },
    { value: 28, label: "28 FPS" },
    { value: 30, label: "30 FPS" },
    { value: 36, label: "36 FPS" },
    { value: 60, label: "60 FPS" }
  ];

  const encoderOptions = [
    { value: "nvenc", label: "NVIDIA Hardware (NVENC)" },
    { value: "vaapi", label: "Intel/AMD Hardware (VA-API)" },
    { value: "x264", label: "Software CPU (x264)" }
  ];

  const micOptions = micList.length > 0 
    ? micList.map(mic => ({ value: mic.id, label: mic.name }))
    : [{ value: "default", label: "Default System Audio" }];

  return (
    <div className="flex-1 p-8 bg-[#09090b] overflow-y-auto">
      <div className="max-w-xl mx-auto">
        <h2 className="text-lg font-medium text-white mb-6">Recording Settings</h2>
        
        <div className="flex flex-col gap-5">
          <div className="flex flex-col gap-1.5">
            <label className="text-[#a1a1aa] text-xs flex items-center gap-1.5"><Folder size={14}/> Output Directory</label>
            <div className="flex gap-2">
              <input type="text" disabled={isDisabled} value={config.output_folder} onChange={(e) => updateConfig("output_folder", e.target.value)}
                className="flex-1 bg-[#18181b] border border-[#27272a] text-white px-3 py-2 text-sm rounded-md outline-none focus:border-[#52525b] transition-colors disabled:opacity-50" />
              <button 
                disabled={isDisabled} 
                onClick={handleChoosePath}
                className="bg-[#27272a] hover:bg-[#3f3f46] text-white px-3 py-2 rounded-md text-sm transition-colors border border-[#3f3f46] flex items-center gap-2 disabled:opacity-50"
              >
                <FolderOpen size={16} /> Browse
              </button>
            </div>
          </div>

          <div className="flex flex-col gap-1.5">
            <label className="text-[#a1a1aa] text-xs flex items-center gap-1.5"><Cpu size={14}/> Video Encoder</label>
            <CustomSelect disabled={isDisabled} value={config.encoder} onChange={(val) => updateConfig("encoder", val)} options={encoderOptions} />
          </div>

          <div className="grid grid-cols-2 gap-5">
            <div className="flex flex-col gap-1.5">
              <label className="text-[#a1a1aa] text-xs flex items-center gap-1.5"><Monitor size={14}/> Resolution</label>
              <CustomSelect disabled={isDisabled} value={config.resolution} onChange={(val) => updateConfig("resolution", val)} options={resolutionOptions} />
            </div>

            <div className="flex flex-col gap-1.5">
              <label className="text-[#a1a1aa] text-xs flex items-center gap-1.5"><Video size={14}/> Frame Rate</label>
              <CustomSelect disabled={isDisabled} value={config.fps} onChange={(val) => updateConfig("fps", val)} options={fpsOptions} />
            </div>
          </div>

          <div className="flex flex-col gap-2">
            <div className="flex justify-between items-center">
              <label className="text-[#a1a1aa] text-xs flex items-center gap-1.5"><Settings size={14}/> Video Bitrate</label>
              <span className="text-[#a1a1aa] text-xs">{config.quality_bitrate} kbps</span>
            </div>
            <input type="range" disabled={isDisabled} min="2000" max="15000" step="500" value={config.quality_bitrate} onChange={(e) => updateConfig("quality_bitrate", Number(e.target.value))}
              className="w-full accent-white disabled:opacity-50 h-1 bg-[#27272a] rounded-lg appearance-none cursor-pointer" />
          </div>

          <div className="flex flex-col gap-4 pt-5 border-t border-[#27272a]">
            <h3 className="text-[#a1a1aa] text-xs font-semibold uppercase tracking-wider mb-1">Display Settings</h3>
            
            <div className="flex items-center justify-between bg-[#18181b] border border-[#27272a] p-3 rounded-md">
              <div className="flex items-center gap-3">
                <MousePointer2 size={18} className="text-[#a1a1aa]" />
                <div>
                  <div className="text-sm text-white">Show Cursor</div>
                  <div className="text-xs text-[#71717a]">Include mouse pointer in recording</div>
                </div>
              </div>
              <Toggle checked={config.show_cursor} onChange={(val) => updateConfig("show_cursor", val)} disabled={isDisabled} />
            </div>
          </div>

          <div className="flex flex-col gap-4 pt-5 border-t border-[#27272a]">
            <h3 className="text-[#a1a1aa] text-xs font-semibold uppercase tracking-wider mb-1">Audio Settings</h3>
            
            <div className="flex items-center justify-between bg-[#18181b] border border-[#27272a] p-3 rounded-md">
              <div className="flex items-center gap-3">
                <Monitor size={18} className="text-[#a1a1aa]" />
                <div>
                  <div className="text-sm text-white">System Audio</div>
                  <div className="text-xs text-[#71717a]">Record internal computer sounds</div>
                </div>
              </div>
              <Toggle checked={config.record_system_audio} onChange={(val) => updateConfig("record_system_audio", val)} disabled={isDisabled} />
            </div>

            <div className="flex items-center justify-between bg-[#18181b] border border-[#27272a] p-3 rounded-md">
              <div className="flex items-center gap-3">
                <Mic size={18} className="text-[#a1a1aa]" />
                <div>
                  <div className="text-sm text-white">Microphone</div>
                  <div className="text-xs text-[#71717a]">Record your voice</div>
                </div>
              </div>
              <Toggle checked={config.record_mic} onChange={(val) => updateConfig("record_mic", val)} disabled={isDisabled} />
            </div>

            {config.record_mic && (
              <div className="flex flex-col gap-1.5 mt-1">
                <label className="text-[#a1a1aa] text-xs flex items-center gap-1.5">Microphone Source</label>
                <CustomSelect disabled={isDisabled} value={config.mic_source} onChange={(val) => updateConfig("mic_source", val)} options={micOptions} />
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}