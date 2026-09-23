/**
 * Keyboard triage for the Pending tab: which key does what, and when a key press is
 * not meant for the inbox at all.
 */

export type TriageCommand = "next" | "prev" | "toggle" | "accept" | "dismiss" | "undo";

const COMMANDS: Record<string, TriageCommand> = {
  j: "next",
  ArrowDown: "next",
  k: "prev",
  ArrowUp: "prev",
  Enter: "toggle",
  o: "toggle",
  a: "accept",
  d: "dismiss",
  u: "undo",
};

/** Marks the row title buttons, where Enter toggles the detail like `o` does. */
export const TRIAGE_ROW_ATTR = "data-triage-row";

function isTyping(el: Element): boolean {
  if (el instanceof HTMLElement && el.isContentEditable) return true;
  return ["INPUT", "TEXTAREA", "SELECT"].includes(el.tagName);
}

/**
 * The command a key press means, or null when it belongs to something else: text
 * being typed, an open dialog, a shortcut with a modifier, or Enter on a button or
 * link that Enter already activates.
 */
export function triageCommand(e: KeyboardEvent): TriageCommand | null {
  if (e.defaultPrevented || e.ctrlKey || e.metaKey || e.altKey) return null;
  const command = COMMANDS[e.key];
  if (!command) return null;
  const target = e.target instanceof Element ? e.target : null;
  if (target && isTyping(target)) return null;
  // A dialog (the scan sheet, a bulk confirmation) owns the keyboard while open.
  if (target?.closest('dialog, [role="dialog"], [role="alertdialog"]')) return null;
  if (
    command === "toggle" &&
    e.key === "Enter" &&
    target?.closest("button, a") &&
    !target.closest(`[${TRIAGE_ROW_ATTR}]`)
  ) {
    return null;
  }
  return command;
}
