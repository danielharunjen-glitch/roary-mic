import React from "react";
import ReactDOM from "react-dom/client";
import AiReplyWindow from "./AiReplyWindow";
import "@/i18n";
import "../App.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <AiReplyWindow />
  </React.StrictMode>,
);
