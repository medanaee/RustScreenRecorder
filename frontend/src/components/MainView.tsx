import { Play, Square, Pause, Monitor, RefreshCw } from "lucide-react";

interface MainViewProps {
  status: "idle" | "recording" | "paused";
  initialized: boolean;
  onInit: () => void;
  onStart: () => void;
  onStop: () => void;
  onPause: () => void;
  onResume: () => void;
}

export function MainView({ status, initialized, onInit, onStart, onStop, onPause, onResume }: MainViewProps) {
  return (
    <div className="flex-1 p-6 flex flex-col items-center justify-center bg-[#09090b]">
      <div className="w-full aspect-video bg-[#18181b] border border-[#27272a] rounded-lg shadow-sm flex flex-col items-center justify-center relative mb-8 overflow-hidden">
        {!initialized ? (
          <>
            <Monitor className="w-16 h-16 text-[#3f3f46]" strokeWidth={1.5} />
            <p className="text-[#71717a] mt-3 text-xs uppercase tracking-wider mb-4">Click to select capture source</p>
            <button onClick={onInit} className="bg-blue-600 hover:bg-blue-500 text-white px-5 py-2 rounded-md font-medium text-sm transition-colors shadow-lg">
              Select Screen to Capture
            </button>
          </>
        ) : (
          <img src="http://127.0.0.1:3001/api/preview_stream" className="w-full h-full object-contain" alt="Live Preview" />
        )}
        
        {initialized && status === "idle" && (
          <button 
            onClick={onInit}
            className="absolute top-4 left-4 flex items-center gap-2 bg-[#09090b]/80 backdrop-blur-sm hover:bg-[#27272a] border border-[#27272a] text-[#a1a1aa] hover:text-white px-3 py-1.5 rounded text-xs font-medium transition-colors"
          >
            <RefreshCw size={14} /> Change Source
          </button>
        )}

        {status !== "idle" && (
          <div className="absolute top-4 right-4 flex items-center gap-2 bg-[#09090b]/80 backdrop-blur-sm px-2.5 py-1.5 rounded border border-[#27272a]">
            <div className={`w-2 h-2 rounded-full ${status === "recording" ? "bg-red-500 animate-pulse" : "bg-yellow-500"}`} />
            <span className="text-[10px] font-medium text-white tracking-wider uppercase">
              {status === "recording" ? "Recording" : "Paused"}
            </span>
          </div>
        )}
      </div>

      <div className="flex items-center gap-3">
        {status === "idle" ? (
          <button onClick={onStart} disabled={!initialized} className="flex items-center gap-2 px-6 py-2 bg-white text-black hover:bg-gray-200 disabled:opacity-50 disabled:cursor-not-allowed rounded-md font-medium text-sm transition-colors">
            <Play size={16} fill="currentColor" /> Start Recording
          </button>
        ) : (
          <div className="flex items-center gap-2 bg-[#18181b] p-1.5 rounded-md border border-[#27272a]">
            {status === "recording" ? (
              <button onClick={onPause} className="flex items-center gap-2 px-4 py-1.5 hover:bg-[#27272a] text-white rounded text-sm transition-colors">
                <Pause size={14} fill="currentColor" /> Pause
              </button>
            ) : (
              <button onClick={onResume} className="flex items-center gap-2 px-4 py-1.5 hover:bg-[#27272a] text-white rounded text-sm transition-colors">
                <Play size={14} fill="currentColor" /> Resume
              </button>
            )}
            <div className="w-px h-4 bg-[#3f3f46]"></div>
            <button onClick={onStop} className="flex items-center gap-2 px-4 py-1.5 text-red-400 hover:text-red-300 hover:bg-red-950/30 rounded text-sm transition-colors">
              <Square size={14} fill="currentColor" /> Stop
            </button>
          </div>
        )}
      </div>
    </div>
  );
}