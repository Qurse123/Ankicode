const codeInput = document.getElementById("pairing-code") as HTMLInputElement;
const statusEl = document.getElementById("status") as HTMLParagraphElement;
const pairButton = document.getElementById("pair") as HTMLButtonElement;
const unpairButton = document.getElementById("unpair") as HTMLButtonElement;
const pairForm = document.getElementById("pair-form") as HTMLDivElement;
const pairedActions = document.getElementById(
  "paired-actions",
) as HTMLDivElement;
const intro = document.getElementById("intro") as HTMLParagraphElement;

function setStatus(text: string): void {
  statusEl.textContent = text;
}

function setPairedUi(paired: boolean): void {
  pairForm.hidden = paired;
  pairedActions.hidden = !paired;
  intro.textContent = paired
    ? "Connected. Accepted submissions will sync while Ankicode is open."
    : "Pair once with the desktop app using the code from Settings. After that, leave this alone.";
}

async function refreshStatus(): Promise<void> {
  const response = (await chrome.runtime.sendMessage({ type: "STATUS" })) as {
    ok?: boolean;
    paired?: boolean;
    online?: boolean;
    clientId?: number | null;
    error?: string;
  };
  if (!response?.ok) {
    setStatus(response?.error || "Unable to read status.");
    return;
  }
  const app = response.online ? "app online" : "app offline — open Ankicode";
  setPairedUi(Boolean(response.paired));
  if (response.paired) {
    setStatus(`Paired · ${app}`);
  } else {
    setStatus(`Not paired · ${app}`);
  }
}

pairButton.addEventListener("click", () => {
  const code = codeInput.value.trim();
  if (!code) {
    setStatus("Enter the pairing code from Ankicode Settings.");
    return;
  }
  pairButton.disabled = true;
  chrome.runtime.sendMessage(
    { type: "PAIR", code },
    (response: { ok?: boolean; error?: string } | undefined) => {
      pairButton.disabled = false;
      if (!response?.ok) {
        setStatus(response?.error || "Pairing failed.");
        return;
      }
      codeInput.value = "";
      void refreshStatus();
    },
  );
});

unpairButton.addEventListener("click", () => {
  chrome.runtime.sendMessage({ type: "UNPAIR" }, () => {
    void refreshStatus();
  });
});

void refreshStatus();
