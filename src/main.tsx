import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { requireRootElement } from "./root";

ReactDOM.createRoot(requireRootElement(document)).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
