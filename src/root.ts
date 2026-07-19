export function requireRootElement(document: Document): HTMLElement {
  const root = document.getElementById("root");

  if (!root) {
    throw new Error('Missing required element "#root".');
  }

  return root;
}
