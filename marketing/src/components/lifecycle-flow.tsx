"use client";

import {
  Background,
  Handle,
  Position,
  ReactFlow,
  ReactFlowProvider,
  type Edge,
  type Node,
  type NodeProps,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";

// Visualizes the four lifecycle verbs as a left-to-right pipeline:
//
//   [SDK call] → [HTTP endpoint] → [storage write] → [webhook fired]
//
// Each row is one verb (click, attribute, identify, convert). Dashed
// arrows between rows convey "these usually happen in funnel order over
// time" without implying they're synchronous.
//
// Hand-positioned (no auto-layout) so the columns line up cleanly and
// reviewers can map each row to the same column type at a glance.

type Tone = "sdk" | "endpoint" | "db" | "webhook";

type StepNodeData = {
  tone: Tone;
  title: string;
  subtitle?: string;
};

type StepFlowNode = Node<StepNodeData, "step">;

const HIDDEN_HANDLE =
  "!w-0 !h-0 !min-w-0 !min-h-0 !border-0 !bg-transparent !opacity-0 !p-0";

const TONE_STYLES: Record<Tone, { border: string; text: string; subtitle: string }> = {
  sdk: {
    border: "border-[#2dd4bf]/40 bg-[#0a3a36]/60",
    text: "text-[#a7f3d0]",
    subtitle: "text-[#5eead4]/70",
  },
  endpoint: {
    border: "border-[#7dd3fc]/40 bg-[#0c1f2e]/60",
    text: "text-[#bae6fd]",
    subtitle: "text-[#7dd3fc]/70",
  },
  db: {
    border: "border-[#fbbf24]/40 bg-[#1f1606]/60",
    text: "text-[#fcd34d]",
    subtitle: "text-[#fbbf24]/70",
  },
  webhook: {
    border: "border-[#a78bfa]/40 bg-[#1a1230]/60",
    text: "text-[#c4b5fd]",
    subtitle: "text-[#a78bfa]/70",
  },
};

function StepNode({ data }: NodeProps<StepFlowNode>) {
  const s = TONE_STYLES[data.tone];
  return (
    <div
      className={`rounded-xl border ${s.border} px-3 py-2 font-mono shadow-[0_4px_18px_rgba(0,0,0,0.25)]`}
    >
      <Handle type="target" position={Position.Left} className={HIDDEN_HANDLE} />
      <Handle type="source" position={Position.Right} className={HIDDEN_HANDLE} />
      <Handle type="target" position={Position.Top} className={HIDDEN_HANDLE} />
      <Handle type="source" position={Position.Bottom} className={HIDDEN_HANDLE} />
      <div className={`text-[12px] font-medium ${s.text}`}>{data.title}</div>
      {data.subtitle ? (
        <div className={`mt-1 whitespace-pre text-[10px] leading-snug ${s.subtitle}`}>
          {data.subtitle}
        </div>
      ) : null}
    </div>
  );
}

const NODE_TYPES = { step: StepNode };

// Column x-coordinates. Tweaked so longer subtitle nodes (DB, webhook)
// don't collide with the next column at our minimum supported width.
const COL_X = { sdk: 0, endpoint: 220, db: 460, webhook: 720 };
const ROW_Y = { click: 0, attribute: 120, identify: 260, convert: 400 };

const nodes: StepFlowNode[] = [
  // CLICK
  {
    id: "sdk-click",
    type: "step",
    position: { x: COL_X.sdk, y: ROW_Y.click },
    data: { tone: "sdk", title: "Rift.click(linkId)", subtitle: "web SDK" },
  },
  {
    id: "ep-click",
    type: "step",
    position: { x: COL_X.endpoint, y: ROW_Y.click },
    data: { tone: "endpoint", title: "POST /v1/lifecycle/click" },
  },
  {
    id: "db-click",
    type: "step",
    position: { x: COL_X.db, y: ROW_Y.click },
    data: { tone: "db", title: "click_events", subtitle: "time-series\nappend-only" },
  },
  {
    id: "wh-click",
    type: "step",
    position: { x: COL_X.webhook, y: ROW_Y.click },
    data: { tone: "webhook", title: "🪝 click", subtitle: "every call" },
  },

  // ATTRIBUTE
  {
    id: "sdk-attr",
    type: "step",
    position: { x: COL_X.sdk, y: ROW_Y.attribute },
    data: { tone: "sdk", title: "rift.attributeLink(linkId)", subtitle: "mobile SDK" },
  },
  {
    id: "ep-attr",
    type: "step",
    position: { x: COL_X.endpoint, y: ROW_Y.attribute },
    data: { tone: "endpoint", title: "POST /v1/lifecycle/attribute" },
  },
  {
    id: "db-attr",
    type: "step",
    position: { x: COL_X.db, y: ROW_Y.attribute },
    data: {
      tone: "db",
      title: "attribution_events",
      subtitle: "time-series + installs\n(first-touch upsert)",
    },
  },
  {
    id: "wh-attr",
    type: "step",
    position: { x: COL_X.webhook, y: ROW_Y.attribute },
    data: {
      tone: "webhook",
      title: "🪝 attribute",
      subtitle: "+ user_id if bound\n+ link_metadata",
    },
  },

  // IDENTIFY
  {
    id: "sdk-id",
    type: "step",
    position: { x: COL_X.sdk, y: ROW_Y.identify },
    data: { tone: "sdk", title: "rift.setUserId(userId)", subtitle: "after signin" },
  },
  {
    id: "ep-id",
    type: "step",
    position: { x: COL_X.endpoint, y: ROW_Y.identify },
    data: { tone: "endpoint", title: "PUT /v1/lifecycle/identify" },
  },
  {
    id: "db-id",
    type: "step",
    position: { x: COL_X.db, y: ROW_Y.identify },
    data: {
      tone: "db",
      title: "installs.$set",
      subtitle: "user_id + identified_at\n(mutable state)",
    },
  },
  {
    id: "wh-id",
    type: "step",
    position: { x: COL_X.webhook, y: ROW_Y.identify },
    data: {
      tone: "webhook",
      title: "🪝 identify",
      subtitle: "once, on transition\n(silent on rebind)",
    },
  },

  // CONVERT
  {
    id: "sdk-conv",
    type: "step",
    position: { x: COL_X.sdk, y: ROW_Y.convert },
    data: { tone: "sdk", title: "rift.trackConversion(...)", subtitle: "valuable action" },
  },
  {
    id: "ep-conv",
    type: "step",
    position: { x: COL_X.endpoint, y: ROW_Y.convert },
    data: { tone: "endpoint", title: "POST /v1/lifecycle/convert" },
  },
  {
    id: "db-conv",
    type: "step",
    position: { x: COL_X.db, y: ROW_Y.convert },
    data: {
      tone: "db",
      title: "conversions",
      subtitle: "time-series\nuser_id → first_link_id",
    },
  },
  {
    id: "wh-conv",
    type: "step",
    position: { x: COL_X.webhook, y: ROW_Y.convert },
    data: { tone: "webhook", title: "🪝 conversion", subtitle: "+ stable event_id" },
  },
];

const ROWS = ["click", "attr", "id", "conv"] as const;

const edges: Edge[] = [
  // Solid arrows within each row: SDK → endpoint → storage → webhook.
  ...ROWS.flatMap((k) => [
    { id: `${k}-1`, source: `sdk-${k}`, target: `ep-${k}` },
    { id: `${k}-2`, source: `ep-${k}`, target: `db-${k}` },
    { id: `${k}-3`, source: `db-${k}`, target: `wh-${k}` },
  ]),
  // Dashed funnel-order arrows: the webhook of one row to the SDK call
  // of the next. These are temporal, not synchronous — they convey
  // "this typically happens later in the user's journey."
  {
    id: "f1",
    source: "wh-click",
    target: "sdk-attr",
    style: { strokeDasharray: "4 4", stroke: "#52525b" },
    animated: true,
  },
  {
    id: "f2",
    source: "wh-attr",
    target: "sdk-id",
    style: { strokeDasharray: "4 4", stroke: "#52525b" },
    animated: true,
  },
  {
    id: "f3",
    source: "wh-id",
    target: "sdk-conv",
    style: { strokeDasharray: "4 4", stroke: "#52525b" },
    animated: true,
  },
];

function Legend() {
  const items: { tone: Tone; label: string }[] = [
    { tone: "sdk", label: "SDK call" },
    { tone: "endpoint", label: "HTTP endpoint" },
    { tone: "db", label: "storage write" },
    { tone: "webhook", label: "webhook fired" },
  ];
  return (
    <div className="flex flex-wrap items-center gap-x-4 gap-y-2 px-4 py-3 text-[12px] text-[#a1a1aa]">
      {items.map((i) => {
        const s = TONE_STYLES[i.tone];
        return (
          <span key={i.tone} className="inline-flex items-center gap-2">
            <span className={`inline-block h-3 w-3 rounded border ${s.border}`} />
            {i.label}
          </span>
        );
      })}
      <span className="inline-flex items-center gap-2 text-[#71717a]">
        <span className="inline-block h-px w-6 border-t border-dashed border-[#52525b]" />
        funnel order
      </span>
    </div>
  );
}

export function LifecycleFlow() {
  return (
    <div className="overflow-hidden rounded-xl border border-[#1e1e22] bg-[#0a0a0c]">
      <div className="h-[560px]">
        <ReactFlowProvider>
          <ReactFlow
            nodes={nodes}
            edges={edges}
            nodeTypes={NODE_TYPES}
            fitView
            fitViewOptions={{ padding: 0.15 }}
            proOptions={{ hideAttribution: true }}
            nodesDraggable={false}
            nodesConnectable={false}
            zoomOnScroll={false}
            zoomOnPinch={false}
            zoomOnDoubleClick={false}
            panOnDrag={false}
            preventScrolling={false}
          >
            <Background gap={24} color="#1e1e22" />
          </ReactFlow>
        </ReactFlowProvider>
      </div>
      <div className="border-t border-[#1e1e22]">
        <Legend />
      </div>
    </div>
  );
}
