import {
  extractProblemMetadata,
  pageShowsAccepted,
  slugFromPathname,
} from "./metadata";

const BUTTON_ID = "ankicode-add-button";

function ensureAddButton(): void {
  if (document.getElementById(BUTTON_ID)) {
    return;
  }
  const metadata = extractProblemMetadata(document);
  if (!metadata) {
    return;
  }

  const button = document.createElement("button");
  button.id = BUTTON_ID;
  button.type = "button";
  button.textContent = "Add to Ankicode";
  button.style.cssText = [
    "position:fixed",
    "bottom:20px",
    "right:20px",
    "z-index:2147483646",
    "padding:10px 14px",
    "border:0",
    "border-radius:8px",
    "background:#1f6f4a",
    "color:#fff",
    "font:600 13px/1.2 system-ui,sans-serif",
    "cursor:pointer",
    "box-shadow:0 6px 18px rgba(0,0,0,0.25)",
  ].join(";");

  button.addEventListener("click", () => {
    const current = extractProblemMetadata(document);
    if (!current) {
      button.textContent = "Could not read problem";
      return;
    }
    button.disabled = true;
    button.textContent = "Adding…";
    chrome.runtime.sendMessage(
      { type: "ADD_PROBLEM", metadata: current },
      (response: { ok?: boolean; error?: string } | undefined) => {
        button.disabled = false;
        if (response?.ok) {
          button.textContent = "Added";
        } else {
          button.textContent = response?.error || "Add failed";
        }
        window.setTimeout(() => {
          button.textContent = "Add to Ankicode";
        }, 2500);
      },
    );
  });

  document.documentElement.appendChild(button);
}

let acceptedPanelVisible = false;
let acceptedInFlight: string | null = null;

function maybeReportAccepted(): void {
  const visible = pageShowsAccepted(document);
  if (!visible) {
    acceptedPanelVisible = false;
    return;
  }
  // Only fire on transition into a visible Accepted result panel.
  if (acceptedPanelVisible) {
    return;
  }
  acceptedPanelVisible = true;

  const slug =
    extractProblemMetadata(document)?.slug ??
    slugFromPathname(window.location.pathname);
  if (!slug || acceptedInFlight === slug) {
    return;
  }
  const storageKey = `ankicode-accepted:${slug}`;
  try {
    if (sessionStorage.getItem(storageKey)) {
      return;
    }
  } catch {
    // sessionStorage may be unavailable.
  }

  acceptedInFlight = slug;
  chrome.runtime.sendMessage(
    { type: "ACCEPTED", slug },
    (response: { ok?: boolean } | undefined) => {
      acceptedInFlight = null;
      if (!response?.ok) {
        // Allow a later transition/retry if enqueue failed.
        acceptedPanelVisible = false;
        return;
      }
      try {
        sessionStorage.setItem(storageKey, "1");
      } catch {
        // ignore
      }
    },
  );
}

function boot(): void {
  ensureAddButton();
  maybeReportAccepted();
  const observer = new MutationObserver(() => {
    ensureAddButton();
    maybeReportAccepted();
  });
  observer.observe(document.documentElement, {
    childList: true,
    subtree: true,
    characterData: true,
  });
}

boot();
document.documentElement.dataset.ankicodeExtension = "ankicode-content-ready";
