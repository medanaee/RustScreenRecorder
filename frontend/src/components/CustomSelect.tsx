import { useState, useRef, useEffect } from "react";
import { ChevronDown, Check } from "lucide-react";

interface Option {
  value: string | number;
  label: string;
}

interface CustomSelectProps {
  options: Option[];
  value: string | number;
  onChange: (value: any) => void;
  disabled?: boolean;
}

export function CustomSelect({ options, value, onChange, disabled }: CustomSelectProps) {
  const [isOpen, setIsOpen] = useState(false);
  const selectRef = useRef<HTMLDivElement>(null);

  const selectedOption = options.find((opt) => opt.value === value) || options[0];

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (selectRef.current && !selectRef.current.contains(event.target as Node)) {
        setIsOpen(false);
      }
    };
    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, []);

  return (
    <div className={`relative ${disabled ? "opacity-50 pointer-events-none" : ""}`} ref={selectRef}>
      <div
        className="flex items-center justify-between bg-[#18181b] border border-[#27272a] text-white px-3 py-2 text-sm rounded-md cursor-pointer hover:border-[#52525b] transition-colors"
        onClick={() => setIsOpen(!isOpen)}
      >
        <span className="truncate">{selectedOption?.label}</span>
        <ChevronDown size={14} className={`text-[#a1a1aa] transition-transform duration-200 ${isOpen ? "rotate-180" : ""}`} />
      </div>
      
      {isOpen && (
        <div className="absolute z-10 w-full mt-1 bg-[#18181b] border border-[#27272a] rounded-md shadow-lg overflow-hidden py-1 max-h-48 overflow-y-auto">
          {options.map((option) => (
            <div
              key={option.value}
              className={`flex items-center justify-between px-3 py-2 text-sm cursor-pointer hover:bg-[#27272a] transition-colors ${
                option.value === value ? "text-white" : "text-[#a1a1aa]"
              }`}
              onClick={() => {
                onChange(option.value);
                setIsOpen(false);
              }}
            >
              <span className="truncate">{option.label}</span>
              {option.value === value && <Check size={14} className="text-white" />}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}