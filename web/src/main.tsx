import React from "react";
import ReactDOM from "react-dom/client";
import { Site } from "./Site";
import "./site.css";
import { App } from "./App";
import "maplibre-gl/dist/maplibre-gl.css";
import "@fontsource-variable/dm-sans/wght.css";
import "@fontsource-variable/space-grotesk/wght.css";
import "./styles.css";
document.body.dataset.surface = window.location.pathname.startsWith(
  "/workspace",
)
  ? "workspace"
  : "site";
ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    {window.location.pathname.startsWith("/workspace") ? <App /> : <Site />}
  </React.StrictMode>,
);
