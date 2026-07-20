const codeInput = document.getElementById("pairing-code") as HTMLInputElement;
const statusEl = document.getElementById("status") as HTMLParagraphElement;
const pairButton = document.getElementById("pair") as HTMLButtonElement;
const unpairButton = document.getElementById("unpair") as HTMLButtonElement;

function setStatus(text: string): void {
  statusEl.textContent = text;
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
  const app = response.online ? "app online" : "app offline";
  if (response.paired) {
    setStatus(`Paired (client ${response.clientId ?? "?"}) · ${app}`);
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
