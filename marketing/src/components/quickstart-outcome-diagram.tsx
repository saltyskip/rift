"use client";

import { useCallback, useEffect, useRef } from "react";
import {
  Background,
  Handle,
  type Node as FlowNode,
  Position,
  ReactFlow,
  ReactFlowProvider,
  useReactFlow,
  type Edge,
  type Node,
  type NodeProps,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";

type LinkNodeData = { href: string };
type PillNodeData = { label: string };
type ClickNodeData = { label: string };
type RouterNodeData = { title: string };
type OutcomeNodeData = {
  eyebrow: string;
  title: string;
  text: string;
};

type LinkFlowNode = FlowNode<LinkNodeData, "link">;
type PillFlowNode = FlowNode<PillNodeData, "pill">;
type ClickFlowNode = FlowNode<ClickNodeData, "click">;
type RouterFlowNode = FlowNode<RouterNodeData, "router">;
type OutcomeFlowNode = FlowNode<OutcomeNodeData, "outcome">;

const HANDLE =
  "!w-0 !h-0 !min-w-0 !min-h-0 !border-0 !bg-transparent !opacity-0 !p-0";

function LinkNode({ data }: NodeProps<LinkFlowNode>) {
  return (
    <div className="rounded-2xl border border-[#2dd4bf]/20 bg-gradient-to-b from-[#0f1d1b] to-[#0d1114] px-5 py-4 text-left shadow-[0_8px_30px_rgba(0,0,0,0.25)]">
      <Handle type="source" position={Position.Bottom} className={HANDLE} />
      <p className="text-[10px] font-semibold uppercase tracking-[0.18em] text-[#2dd4bf]">
        Branded Link
      </p>
      <p className="mt-2.5 whitespace-nowrap text-lg font-semibold leading-snug text-[#f0f0f0]">
        {data.href}
      </p>
    </div>
  );
}

function PillNode({ data }: NodeProps<PillFlowNode>) {
  return (
    <div className="w-[90px] rounded-full border border-[#1e293b] bg-gradient-to-b from-[#0f1520] to-[#0c0e13] px-4 py-2 text-center shadow-[0_4px_16px_rgba(0,0,0,0.2)]">
      <Handle type="target" position={Position.Top} className={HANDLE} />
      <Handle type="source" position={Position.Bottom} className={HANDLE} />
      <p className="text-[12px] font-medium text-[#c8d2df]">{data.label}</p>
    </div>
  );
}

function ClickNode({ data }: NodeProps<ClickFlowNode>) {
  return (
    <div className="rounded-2xl border border-[#2dd4bf]/15 bg-gradient-to-b from-[#0f1d1b] to-[#0d1114] px-5 py-3 text-center shadow-[0_6px_20px_rgba(0,0,0,0.2)]">
      <Handle type="target" position={Position.Top} className={HANDLE} />
      <Handle type="source" position={Position.Bottom} className={HANDLE} />
      <p className="text-[10px] font-semibold uppercase tracking-[0.18em] text-[#2dd4bf]/70">
        {data.label}
      </p>
    </div>
  );
}

function RouterNode({ data }: NodeProps<RouterFlowNode>) {
  return (
    <div className="w-[220px] rounded-2xl border border-[#1a2030] bg-gradient-to-b from-[#101420] to-[#0c0e14] px-5 py-4 text-center shadow-[0_8px_24px_rgba(0,0,0,0.2)]">
      <Handle type="target" position={Position.Top} className={HANDLE} />
      <Handle type="source" position={Position.Bottom} className={HANDLE} />
      <p className="text-[10px] font-semibold uppercase tracking-[0.18em] text-[#7dd3fc]">
        Rift
      </p>
      <p className="mt-2 text-base font-semibold text-[#f0f0f0]">
        {data.title}
      </p>
    </div>
  );
}

function OutcomeNode({ data }: NodeProps<OutcomeFlowNode>) {
  return (
    <div className="w-[160px] rounded-2xl border border-[#1a2030] bg-gradient-to-b from-[#101218] to-[#0c0d12] px-4 py-3.5 text-left shadow-[0_8px_24px_rgba(0,0,0,0.2)]">
      <Handle type="target" position={Position.Top} className={HANDLE} />
      <p
        className={`text-[10px] font-semibold uppercase tracking-[0.18em] ${data.text}`}
      >
        {data.eyebrow}
      </p>
      <p className="mt-1.5 text-[14px] font-semibold leading-snug text-[#f0f0f0]">
        {data.title}
      </p>
    </div>
  );
}

const nodeTypes = {
  link: LinkNode,
  pill: PillNode,
  click: ClickNode,
  router: RouterNode,
  outcome: OutcomeNode,
};

/* ---- layout constants ----
 *  Rows: link(0) → pills(140) → click(260) → router(370) → outcomes(520)
 *  6 channel pills fan out, 4 outcome nodes fan out
 */
const CX = 400;

const CHANNELS = ["Ads", "Email", "Social", "SMS", "QR Code", "Website"];
const PILL_W = 90;
const PILL_GAP = 22;
const PILL_TOTAL = CHANNELS.length * PILL_W + (CHANNELS.length - 1) * PILL_GAP;
const PILL_START = CX - PILL_TOTAL / 2;

const NODE_DEFAULTS = { draggable: false, selectable: false } as const;

const pillNodes: Node[] = CHANNELS.map((label, i) => ({
  id: `ch-${label.toLowerCase().replace(" ", "-")}`,
  type: "pill" as const,
  position: { x: PILL_START + i * (PILL_W + PILL_GAP), y: 140 },
  ...NODE_DEFAULTS,
  data: { label },
}));

const OUTCOME_W = 160;
const OUTCOME_GAP = 28;
const OUTCOMES = [
  { id: "ios", eyebrow: "iPhone", title: "App or App Store", text: "text-[#7dd3fc]", accent: "#7dd3fc" },
  { id: "android", eyebrow: "Android", title: "App or Play Store", text: "text-[#fbbf24]", accent: "#fbbf24" },
  { id: "web", eyebrow: "Web", title: "Web URL", text: "text-[#f472b6]", accent: "#f472b6" },
  { id: "agent", eyebrow: "Agent", title: "API or MCP", text: "text-[#a78bfa]", accent: "#a78bfa" },
];
const OUTCOME_TOTAL = OUTCOMES.length * OUTCOME_W + (OUTCOMES.length - 1) * OUTCOME_GAP;
const OUTCOME_START = CX - OUTCOME_TOTAL / 2;

const outcomeNodes: Node[] = OUTCOMES.map((o, i) => ({
  id: o.id,
  type: "outcome" as const,
  position: { x: OUTCOME_START + i * (OUTCOME_W + OUTCOME_GAP), y: 520 },
  ...NODE_DEFAULTS,
  data: { eyebrow: o.eyebrow, title: o.title, text: o.text },
}));

const nodes: Node[] = [
  {
    id: "link",
    type: "link",
    position: { x: CX - 180, y: 0 },
    ...NODE_DEFAULTS,
    data: { href: "go.yourcompany.com/summer-sale" },
  },
  ...pillNodes,
  {
    id: "click",
    type: "click",
    position: { x: CX - 50, y: 260 },
    ...NODE_DEFAULTS,
    data: { label: "Click" },
  },
  {
    id: "router",
    type: "router",
    position: { x: CX - 110, y: 370 },
    ...NODE_DEFAULTS,
    data: { title: "Route by platform" },
  },
  ...outcomeNodes,
];

const EDGE_DEFAULTS = {
  animated: true,
  style: { strokeWidth: 1.5 },
} as const;

const pillEdgesIn: Edge[] = CHANNELS.map((label) => {
  const id = `ch-${label.toLowerCase().replace(" ", "-")}`;
  return {
    id: `link-${id}`,
    source: "link",
    target: id,
    type: "smoothstep",
    ...EDGE_DEFAULTS,
    style: { ...EDGE_DEFAULTS.style, stroke: "#2dd4bf30" },
  };
});

const pillEdgesOut: Edge[] = CHANNELS.map((label) => {
  const id = `ch-${label.toLowerCase().replace(" ", "-")}`;
  return {
    id: `${id}-click`,
    source: id,
    target: "click",
    type: "smoothstep",
    ...EDGE_DEFAULTS,
    style: { ...EDGE_DEFAULTS.style, stroke: "#2dd4bf30" },
  };
});

const outcomeEdges: Edge[] = OUTCOMES.map((o) => ({
  id: `router-${o.id}`,
  source: "router",
  target: o.id,
  type: "smoothstep",
  ...EDGE_DEFAULTS,
  style: { ...EDGE_DEFAULTS.style, stroke: `${o.accent}50` },
}));

const edges: Edge[] = [
  ...pillEdgesIn,
  ...pillEdgesOut,
  {
    id: "click-router",
    source: "click",
    target: "router",
    ...EDGE_DEFAULTS,
    style: { ...EDGE_DEFAULTS.style, stroke: "#2dd4bf40" },
  },
  ...outcomeEdges,
];

function FitOnResize() {
  const { fitView } = useReactFlow();
  const containerRef = useRef<HTMLDivElement | null>(null);

  const refit = useCallback(() => {
    fitView({ padding: 0.08 });
  }, [fitView]);

  useEffect(() => {
    const el = containerRef.current?.closest(".react-flow") as HTMLElement | null;
    if (!el) return;
    const ro = new ResizeObserver(refit);
    ro.observe(el);
    return () => ro.disconnect();
  }, [refit]);

  return <div ref={containerRef} />;
}

function DiagramFlow() {
  return (
    <ReactFlow
      nodes={nodes}
      edges={edges}
      nodeTypes={nodeTypes}
      fitView
      fitViewOptions={{ padding: 0.08 }}
      proOptions={{ hideAttribution: true }}
      nodesDraggable={false}
      nodesConnectable={false}
      elementsSelectable={false}
      zoomOnScroll={false}
      panOnDrag
      zoomOnPinch
      zoomOnDoubleClick={false}
      preventScrolling={false}
    >
      <Background
        gap={32}
        size={0.8}
        color="rgba(148, 163, 184, 0.04)"
      />
      <FitOnResize />
    </ReactFlow>
  );
}

export function QuickstartOutcomeDiagram() {
  return (
    <section className="space-y-5">
      <div className="flex flex-col gap-3 md:flex-row md:items-end md:justify-between">
        <div>
          <p className="text-[11px] font-semibold uppercase tracking-[0.18em] text-[#2dd4bf]">
            What You End Up With
          </p>
          <h2 className="mt-3 text-2xl font-bold text-[#fafafa]">
            One branded link, routed everywhere
          </h2>
          <p className="mt-2 max-w-2xl text-[14px] leading-relaxed text-[#8f96a3]">
            One public URL for people to click, with Rift routing each tap to
            the right destination.
          </p>
        </div>
        <div className="flex flex-wrap gap-2 text-[12px] text-[#d4d4d8]">
          <span className="rounded-full border border-[#2dd4bf]/20 bg-[#2dd4bf]/10 px-3 py-1">
            click tracked
          </span>
          <span className="rounded-full border border-[#60a5fa]/20 bg-[#60a5fa]/10 px-3 py-1">
            attribution ready
          </span>
          <span className="rounded-full border border-[#f59e0b]/20 bg-[#f59e0b]/10 px-3 py-1">
            agent-readable
          </span>
        </div>
      </div>

      <div className="rounded-[22px] border border-[#1a1c21] bg-gradient-to-b from-[#0f1217] to-[#0b0d11] p-3 md:p-4">
        <div className="overflow-hidden rounded-2xl border border-[#141820] bg-[radial-gradient(ellipse_at_top,#0e181e_0%,#0c0f14_50%,#0b0d11_100%)]">
          <div className="h-[680px] w-full">
            <ReactFlowProvider>
              <DiagramFlow />
            </ReactFlowProvider>
          </div>
        </div>
      </div>
    </section>
  );
}
