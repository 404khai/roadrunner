import React from "react";
import ReactDOM from "react-dom/client";
import { App } from "./App";
import "maplibre-gl/dist/maplibre-gl.css";
import "@fontsource-variable/dm-sans/wght.css";
import "@fontsource-variable/space-grotesk/wght.css";
import "./styles.css";
ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
