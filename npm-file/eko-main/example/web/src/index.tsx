import React from "react";
import App from "./App.tsx";
import ReactDOM from "react-dom/client";
import "./index.css";

const root = ReactDOM.createRoot(document.getElementById("root") as HTMLElement);
root.render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);

// Login automation testing
import { auto_test_case } from "./main.ts";
setTimeout(async () => {
  await auto_test_case();
}, 500);
