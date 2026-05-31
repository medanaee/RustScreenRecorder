import { useState, useEffect } from "react";
import { Sidebar } from "./components/Sidebar";
import { MainView } from "./components/MainView";
import { SettingsView } from "./components/SettingsView";
import type { RecordConfig } from "./types";

const API_URL = "http://127.0.0.1:3001/api";

export default function App() {
  const [status, setStatus] = useState<"idle" | "recording" | "paused">("idle");
  const [activeTab, setActiveTab] = useState<"home" | "settings">("home");
  const [initialized, setInitialized] = useState(false);
  
  const [config, setConfig] = useState<RecordConfig>(() => {
    const saved = localStorage.getItem("r_screen_rec_config");
    if (saved) {
      try {
        const parsed = JSON.parse(saved);
        if (!parsed.encoder) parsed.encoder = "x264";
        return parsed;
      } catch (e) {}
    }
    return {
      fps: 30,
      quality_bitrate: 4000,
      resolution: "original",
      encoder: "x264",
      record_mic: true,
      record_system_audio: true,
      mic_source: "default",
      output_folder: "/tmp",
      show_cursor: true,
    };
  });

  useEffect(() => {
    localStorage.setItem("r_screen_rec_config", JSON.stringify(config));
  }, [config]);

  useEffect(() => {
    fetch(`${API_URL}/status`)
      .then((r) => r.text())
      .then((res) => setStatus(res as any))
      .catch(() => console.error("Backend offline"));
      
    fetch(`${API_URL}/default_path`)
      .then((r) => r.text())
      .then((path) => setConfig((c) => c.output_folder === "/tmp" ? { ...c, output_folder: path } : c))
      .catch(console.error);
  }, []);

  const handleInit = async () => {
    await apiCall("init", { show_cursor: config.show_cursor });
    setInitialized(true);
  };

  const apiCall = async (endpoint: string, body?: any) => {
    await fetch(`${API_URL}/${endpoint}`, { 
      method: "POST", 
      headers: { "Content-Type": "application/json" }, 
      body: body ? JSON.stringify(body) : undefined 
    });
  }

  return (
    <div className="flex h-screen w-screen bg-[#09090b] text-white font-sans overflow-hidden selection:bg-[#27272a]">
      <Sidebar 
        activeTab={activeTab} 
        setActiveTab={setActiveTab} 
        onExit={() => { apiCall("exit"); window.close(); }} 
      />
      {activeTab === "home" ? (
        <MainView 
          status={status} 
          initialized={initialized}
          onInit={handleInit}
          onStart={() => { apiCall("start", config); setStatus("recording"); }} 
          onStop={() => { apiCall("stop"); setStatus("idle"); }} 
          onPause={() => { apiCall("pause"); setStatus("paused"); }} 
          onResume={() => { apiCall("resume"); setStatus("recording"); }} 
        />
      ) : (
        <SettingsView config={config} setConfig={setConfig} status={status} />
      )}
    </div>
  );
}