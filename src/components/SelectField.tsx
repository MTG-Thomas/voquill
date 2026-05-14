import type { JSX } from "preact";
import { useEffect, useMemo, useRef, useState } from "preact/hooks";
import { IconCheck, IconChevronDown } from "@tabler/icons-preact";
import { tokens } from "../design-tokens.ts";

export interface SelectOption {
  value: string;
  label: string;
  disabled?: boolean;
  searchText?: string;
}

interface SelectFieldProps {
  value: string;
  options: SelectOption[];
  onChange: (value: string) => void;
  placeholder?: string;
  disabled?: boolean;
  searchable?: boolean;
  searchPlaceholder?: string;
  emptyMessage?: string;
  className?: string;
  style?: JSX.CSSProperties;
  ariaLabel?: string;
}

export function SelectField({
  value,
  options,
  onChange,
  placeholder = "Select an option",
  disabled = false,
  searchable = false,
  searchPlaceholder = "Search...",
  emptyMessage = "No options found",
  className = "",
  style,
  ariaLabel,
}: SelectFieldProps) {
  const [isOpen, setIsOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const [highlightedIndex, setHighlightedIndex] = useState(-1);
  const [isTriggerHovered, setIsTriggerHovered] = useState(false);
  const [isTriggerFocused, setIsTriggerFocused] = useState(false);

  const containerRef = useRef<HTMLDivElement | null>(null);
  const triggerRef = useRef<HTMLButtonElement | null>(null);
  const searchInputRef = useRef<HTMLInputElement | null>(null);
  const listboxIdRef = useRef(`voquill-select-listbox-${Math.random().toString(36).slice(2, 11)}`);

  const selectedOption = options.find((option) => option.value === value) || null;

  const filteredOptions = useMemo(() => {
    if (!searchable) {
      return options;
    }

    const query = searchQuery.trim().toLowerCase();
    if (!query) {
      return options;
    }

    return options.filter((option) => {
      const searchPool = `${option.label} ${option.value} ${option.searchText || ""}`.toLowerCase();
      return searchPool.includes(query);
    });
  }, [options, searchable, searchQuery]);

  const findNextEnabledIndex = (startIndex: number, direction: 1 | -1) => {
    if (filteredOptions.length === 0 || filteredOptions.every((option) => option.disabled)) {
      return -1;
    }

    let index = startIndex;
    for (let step = 0; step < filteredOptions.length; step += 1) {
      index = (index + direction + filteredOptions.length) % filteredOptions.length;
      if (!filteredOptions[index].disabled) {
        return index;
      }
    }

    return -1;
  };

  const closeDropdown = (focusTrigger: boolean) => {
    setIsOpen(false);
    setSearchQuery("");
    setHighlightedIndex(-1);
    if (focusTrigger) {
      triggerRef.current?.focus();
    }
  };

  const openDropdown = () => {
    if (disabled) {
      return;
    }
    setIsOpen(true);
  };

  const selectOption = (optionValue: string) => {
    const option = options.find((candidate) => candidate.value === optionValue);
    if (!option || option.disabled) {
      return;
    }
    onChange(optionValue);
    closeDropdown(true);
  };

  useEffect(() => {
    if (!isOpen) {
      return;
    }

    const handleOutsidePointer = (event: PointerEvent) => {
      const target = event.target as Node | null;
      if (!target || !containerRef.current?.contains(target)) {
        closeDropdown(false);
      }
    };

    window.addEventListener("pointerdown", handleOutsidePointer);
    return () => {
      window.removeEventListener("pointerdown", handleOutsidePointer);
    };
  }, [isOpen]);

  useEffect(() => {
    if (!isOpen) {
      return;
    }

    if (searchable) {
      requestAnimationFrame(() => {
        searchInputRef.current?.focus();
      });
      return;
    }

    const selectedIndex = filteredOptions.findIndex(
      (option) => option.value === value && !option.disabled,
    );
    if (selectedIndex >= 0) {
      setHighlightedIndex(selectedIndex);
      return;
    }

    setHighlightedIndex(findNextEnabledIndex(-1, 1));
  }, [isOpen, searchable, filteredOptions, value]);

  useEffect(() => {
    if (!isOpen || !searchable) {
      return;
    }

    const selectedIndex = filteredOptions.findIndex(
      (option) => option.value === value && !option.disabled,
    );
    if (selectedIndex >= 0) {
      setHighlightedIndex(selectedIndex);
      return;
    }

    setHighlightedIndex(findNextEnabledIndex(-1, 1));
  }, [searchQuery, isOpen, searchable, filteredOptions, value]);

  useEffect(() => {
    if (!isOpen || highlightedIndex < 0) {
      return;
    }

    const highlightedOption = containerRef.current?.querySelector<HTMLButtonElement>(
      `[data-option-index="${highlightedIndex}"]`,
    );
    highlightedOption?.scrollIntoView({ block: "nearest" });
  }, [highlightedIndex, isOpen]);

  const handleKeyDown = (event: KeyboardEvent) => {
    if (disabled) {
      return;
    }

    if (!isOpen) {
      if (
        event.key === "ArrowDown" ||
        event.key === "ArrowUp" ||
        event.key === "Enter" ||
        event.key === " "
      ) {
        event.preventDefault();
        openDropdown();
      }
      return;
    }

    if (event.key === "Escape") {
      event.preventDefault();
      closeDropdown(true);
      return;
    }

    if (event.key === "Tab") {
      closeDropdown(false);
      return;
    }

    if (event.key === "ArrowDown") {
      event.preventDefault();
      setHighlightedIndex((index) => findNextEnabledIndex(index, 1));
      return;
    }

    if (event.key === "ArrowUp") {
      event.preventDefault();
      setHighlightedIndex((index) => findNextEnabledIndex(index, -1));
      return;
    }

    if (event.key === "Home") {
      event.preventDefault();
      setHighlightedIndex(findNextEnabledIndex(-1, 1));
      return;
    }

    if (event.key === "End") {
      event.preventDefault();
      setHighlightedIndex(findNextEnabledIndex(0, -1));
      return;
    }

    if (event.key === "Enter") {
      event.preventDefault();
      if (highlightedIndex < 0) {
        return;
      }
      const option = filteredOptions[highlightedIndex];
      if (!option?.disabled) {
        selectOption(option.value);
      }
    }
  };

  const triggerStyle: JSX.CSSProperties = {
    width: "100%",
    background: isTriggerHovered && !disabled ? tokens.colors.bgHover : tokens.colors.bgSecondary,
    color: tokens.colors.textPrimary,
    border: `1px solid ${isOpen || isTriggerFocused ? tokens.colors.accentPrimary : "rgba(255, 255, 255, 0.12)"}`,
    borderBottom: `2px solid ${isOpen || isTriggerFocused ? tokens.colors.accentPrimary : "rgba(255, 255, 255, 0.42)"}`,
    borderRadius: tokens.radii.input,
    padding: "8px 10px",
    fontSize: tokens.typography.sizeSm,
    textAlign: "left",
    display: "flex",
    alignItems: "center",
    justifyContent: "space-between",
    gap: tokens.spacing.sm,
    cursor: disabled ? "not-allowed" : "pointer",
    transition: "border-color 0.2s ease, box-shadow 0.2s ease, background-color 0.2s ease",
    opacity: disabled ? 0.55 : 1,
    boxShadow: isOpen || isTriggerFocused ? tokens.shadows.accent : "none",
  };

  const menuStyle: JSX.CSSProperties = {
    position: "absolute",
    top: "calc(100% + 6px)",
    left: 0,
    width: "100%",
    zIndex: 120,
    border: "1px solid rgba(255, 255, 255, 0.12)",
    borderRadius: tokens.radii.panel,
    background: tokens.colors.glassBgHeavy,
    boxShadow: tokens.shadows.lg,
    backdropFilter: `blur(${tokens.colors.glassBlur})`,
    WebkitBackdropFilter: `blur(${tokens.colors.glassBlur})`,
    overflow: "hidden",
  };

  const optionBaseStyle: JSX.CSSProperties = {
    width: "100%",
    border: "1px solid transparent",
    borderRadius: "8px",
    background: "transparent",
    color: tokens.colors.textPrimary,
    padding: "8px 10px",
    fontSize: tokens.typography.sizeSm,
    textAlign: "left",
    display: "flex",
    alignItems: "center",
    justifyContent: "space-between",
    gap: tokens.spacing.sm,
    cursor: "pointer",
    transition: "background-color 0.14s ease, border-color 0.14s ease",
  };

  return (
    <div
      ref={containerRef}
      className={className}
      style={{ position: "relative", width: "100%", minWidth: 0, flex: "1 1 auto", ...style }}
      onKeyDown={handleKeyDown}
    >
      <button
        ref={triggerRef}
        type="button"
        role="combobox"
        aria-expanded={isOpen}
        aria-haspopup="listbox"
        aria-controls={listboxIdRef.current}
        aria-label={ariaLabel}
        disabled={disabled}
        style={triggerStyle}
        onClick={() => {
          if (isOpen) {
            closeDropdown(false);
            return;
          }
          openDropdown();
        }}
        onMouseEnter={() => setIsTriggerHovered(true)}
        onMouseLeave={() => setIsTriggerHovered(false)}
        onFocus={() => setIsTriggerFocused(true)}
        onBlur={() => setIsTriggerFocused(false)}
      >
        <span
          style={{
            display: "block",
            flex: 1,
            minWidth: 0,
            whiteSpace: "nowrap",
            overflow: "hidden",
            textOverflow: "ellipsis",
            color: selectedOption ? tokens.colors.textPrimary : tokens.colors.textMuted,
          }}
        >
          {selectedOption?.label || placeholder}
        </span>
        <IconChevronDown
          size={16}
          style={{
            color: tokens.colors.textSecondary,
            flexShrink: 0,
            transform: isOpen ? "rotate(180deg)" : "rotate(0deg)",
            transition: "transform 0.2s ease",
          }}
        />
      </button>

      {isOpen && (
        <div role="listbox" id={listboxIdRef.current} style={menuStyle}>
          {searchable && (
            <div
              style={{
                padding: tokens.spacing.sm,
                borderBottom: "1px solid rgba(255, 255, 255, 0.08)",
              }}
            >
              <input
                ref={searchInputRef}
                type="text"
                value={searchQuery}
                onInput={(event) => setSearchQuery((event.target as HTMLInputElement).value)}
                placeholder={searchPlaceholder}
                style={{
                  width: "100%",
                  background: tokens.colors.bgSecondary,
                  color: tokens.colors.textPrimary,
                  border: "1px solid rgba(255, 255, 255, 0.12)",
                  borderRadius: tokens.radii.input,
                  padding: "8px 10px",
                  fontSize: tokens.typography.sizeSm,
                  outline: "none",
                }}
              />
            </div>
          )}

          <div style={{ maxHeight: "260px", overflow: "auto", padding: "6px" }}>
            {filteredOptions.length === 0 ? (
              <div
                style={{
                  color: tokens.colors.textSecondary,
                  fontSize: tokens.typography.sizeSm,
                  textAlign: "center",
                  padding: "10px 8px",
                }}
              >
                {emptyMessage}
              </div>
            ) : (
              filteredOptions.map((option, index) => {
                const isSelected = option.value === value;
                const isHighlighted = index === highlightedIndex;
                const isInteractive = !option.disabled;

                const optionStyle: JSX.CSSProperties = {
                  ...optionBaseStyle,
                  cursor: isInteractive ? "pointer" : "not-allowed",
                  opacity: isInteractive ? 1 : 0.5,
                  background: isSelected
                    ? "rgba(96, 205, 255, 0.16)"
                    : isHighlighted && isInteractive
                      ? tokens.colors.bgHover
                      : "transparent",
                  borderColor: isSelected
                    ? "rgba(96, 205, 255, 0.44)"
                    : isHighlighted && isInteractive
                      ? "rgba(255, 255, 255, 0.08)"
                      : "transparent",
                };

                return (
                  <button
                    key={option.value}
                    type="button"
                    role="option"
                    aria-selected={isSelected}
                    data-option-index={index}
                    disabled={option.disabled}
                    style={optionStyle}
                    onMouseEnter={() => {
                      if (!option.disabled) {
                        setHighlightedIndex(index);
                      }
                    }}
                    onClick={() => selectOption(option.value)}
                  >
                    <span
                      style={{
                        minWidth: 0,
                        overflow: "hidden",
                        textOverflow: "ellipsis",
                        whiteSpace: "nowrap",
                      }}
                    >
                      {option.label}
                    </span>
                    {isSelected && <IconCheck size={14} />}
                  </button>
                );
              })
            )}
          </div>
        </div>
      )}
    </div>
  );
}
