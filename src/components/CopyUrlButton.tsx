"use client";

import { useState } from "react";

export default function CopyUrlButton() {
  const [copied, setCopied] = useState(false);

  function copy() {
    navigator.clipboard.writeText(location.href).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    });
  }

  return (
    <button
      type="button"
      onClick={copy}
      className="text-sm font-bold text-da-blue-900 hover:text-da-blue-600"
    >
      {copied ? "コピーしました" : "このページのURLをコピー"}
    </button>
  );
}
