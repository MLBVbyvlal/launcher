import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import ConsoleWindow from "./ConsoleWindow";

const root = ReactDOM.createRoot(document.getElementById("root") as HTMLElement);
const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

function renderMain() {
  root.render(<React.StrictMode><App /></React.StrictMode>);
}

function renderConsole() {
  root.render(<ConsoleWindow />);
}

if (isTauri) {
  import("@tauri-apps/api/core").then(({ invoke }) => {
    invoke<string>("get_window_type")
      .then(type => { type === "console" ? renderConsole() : renderMain() })
      .catch(() => renderMain());
  }).catch(() => renderMain());
} else {
  renderMain();
}
