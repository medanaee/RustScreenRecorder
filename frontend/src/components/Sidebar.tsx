import { Settings, Home, LogOut, Video } from "lucide-react";

interface SidebarProps {
  activeTab: string;
  setActiveTab: (tab: "home" | "settings") => void;
  onExit: () => void;
}

export function Sidebar({ activeTab, setActiveTab, onExit }: SidebarProps) {
  return (
    <div className="w-56 bg-[#09090b] border-r border-[#27272a] flex flex-col justify-between flex-shrink-0">
      <div>
        <div className="flex items-center gap-2 px-5 py-4 border-b border-[#27272a]">
          <Video size={18} strokeWidth={2} className="text-white" />
          <h1 className="text-white text-xs font-semibold tracking-wider uppercase">Wayland Rec</h1>
        </div>
        <nav className="flex flex-col gap-1 p-3">
          <button
            onClick={() => setActiveTab("home")}
            className={`flex items-center gap-2.5 px-3 py-2 rounded-md text-sm transition-colors ${
              activeTab === "home" ? "bg-[#27272a] text-white" : "text-[#a1a1aa] hover:text-white hover:bg-[#18181b]"
            }`}
          >
            <Home size={16} /> Record
          </button>
          <button
            onClick={() => setActiveTab("settings")}
            className={`flex items-center gap-2.5 px-3 py-2 rounded-md text-sm transition-colors ${
              activeTab === "settings" ? "bg-[#27272a] text-white" : "text-[#a1a1aa] hover:text-white hover:bg-[#18181b]"
            }`}
          >
            <Settings size={16} /> Settings
          </button>
        </nav>
      </div>
      <div className="p-3 border-t border-[#27272a]">
        <button
          onClick={onExit}
          className="w-full flex items-center gap-2.5 px-3 py-2 rounded-md text-sm text-[#f87171] hover:bg-red-500/10 transition-colors"
        >
          <LogOut size={16} /> Exit
        </button>
      </div>
    </div>
  );
}