"use client";

export type Span = "all" | "30d" | "7d";

const OPTS: { v: Span; label: string }[] = [
  { v: "all", label: "全期間" },
  { v: "30d", label: "30日" },
  { v: "7d", label: "7日" },
];

export default function SpanToggle({
  span,
  onChange,
}: {
  span: Span;
  onChange: (s: Span) => void;
}) {
  return (
    <div className="inline-flex border border-da-gray-200 text-xs font-bold">
      {OPTS.map((o, i) => (
        <button
          key={o.v}
          type="button"
          onClick={() => onChange(o.v)}
          className={`px-4 py-2 ${i > 0 ? "border-l border-da-gray-200" : ""} ${
            span === o.v
              ? "bg-da-blue-1200 text-white"
              : "bg-white text-da-gray-600 hover:bg-da-blue-50"
          }`}
        >
          {o.label}
        </button>
      ))}
    </div>
  );
}
