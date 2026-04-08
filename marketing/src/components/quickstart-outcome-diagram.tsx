"use client";

import {
  Background,
  Handle,
  type Node as FlowNode,
  Position,
  ReactFlow,
  type Edge,
  type Node,
  type NodeProps,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";

type LinkNodeData = { href: string; body: string };
type ChannelNodeData = { title: string; items: string[] };
type RouterNodeData = { title: string; body: string };
type OutcomeNodeData = {
  eyebrow: string;
  title: string;
  body: string;
  accent: string;
  text: string;
};

type LinkFlowNode = FlowNode<LinkNodeData, "link">;
type ChannelFlowNode = FlowNode<ChannelNodeData, "channel">;
type RouterFlowNode = FlowNode<RouterNodeData, "router">;
type OutcomeFlowNode = FlowNode<OutcomeNodeData, "outcome">;

const HANDLE =
  "!w-0 !h-0 !min-w-0 !min-h-0 !border-0 !bg-transparent !opacity-0 !p-0";

function LinkNode({ data }: NodeProps<LinkFlowNode>) {
  return (
    <div className="w-[300px] rounded-2xl border border-[#2dd4bf]/20 bg-gradient-to-b from-[#0f1d1b] to-[#0d1114] px-5 py-4 text-left shadow-[0_8px_30px_rgba(0,0,0,0.25)]">
      <Handle type="source" position={Position.Bottom} className={HANDLE} />
      <p className="text-[10px] font-semibold uppercase tracking-[0.18em] text-[#2dd4bf]">
        Branded Link
      </p>
      <p className="mt-2.5 text-lg font-semibold leading-snug text-[#f0f0f0]">
        {data.href}
      </p>
      <p className="mt-1.5 text-[13px] leading-relaxed text-[#8a9e98]">
        {data.body}
      </p>
    </div>
  );
}

function ChannelNode({ data }: NodeProps<ChannelFlowNode>) {
  return (
    <div className="w-[300px] rounded-2xl border border-[#1a2030] bg-gradient-to-b from-[#0f1319] to-[#0c0e13] px-5 py-4 text-left shadow-[0_8px_24px_rgba(0,0,0,0.2)]">
      <Handle type="target" position={Position.Top} className={HANDLE} />
      <Handle type="source" position={Position.Bottom} className={HANDLE} />
      <p className="text-[10px] font-semibold uppercase tracking-[0.18em] text-[#64748b]">
        {data.title}
      </p>
      <div className="mt-2.5 flex flex-wrap gap-1.5">
        {data.items.map((item) => (
          <span
            key={item}
            className="rounded-full border border-[#1e293b] bg-[#0f1520] px-2.5 py-1 text-[11px] text-[#c8d2df]"
          >
            {item}
          </span>
        ))}
      </div>
    </div>
  );
}

function RouterNode({ data }: NodeProps<RouterFlowNode>) {
  return (
    <div className="w-[260px] rounded-2xl border border-[#1a2030] bg-gradient-to-b from-[#101420] to-[#0c0e14] px-5 py-4 text-center shadow-[0_8px_24px_rgba(0,0,0,0.2)]">
      <Handle type="target" position={Position.Top} className={HANDLE} />
      <Handle type="source" position={Position.Bottom} className={HANDLE} />
      <p className="text-[10px] font-semibold uppercase tracking-[0.18em] text-[#7dd3fc]">
        Rift
      </p>
      <p className="mt-2 text-base font-semibold text-[#f0f0f0]">
        {data.title}
      </p>
      <p className="mt-1.5 text-[12px] leading-relaxed text-[#8892a4]">
        {data.body}
      </p>
    </div>
  );
}

function OutcomeNode({ data }: NodeProps<OutcomeFlowNode>) {
  return (
    <div className="w-[200px] rounded-2xl border border-[#1a2030] bg-gradient-to-b from-[#101218] to-[#0c0d12] px-4 py-4 text-left shadow-[0_8px_24px_rgba(0,0,0,0.2)]">
      <Handle type="target" position={Position.Top} className={HANDLE} />
      <p
        className={`text-[10px] font-semibold uppercase tracking-[0.18em] ${data.text}`}
      >
        {data.eyebrow}
      </p>
      <p className="mt-2 text-[15px] font-semibold leading-snug text-[#f0f0f0]">
        {data.title}
      </p>
      <p className="mt-1.5 text-[12px] leading-relaxed text-[#8892a4]">
        {data.body}
      </p>
    </div>
  );
}

const nodeTypes = {
  link: LinkNode,
  channel: ChannelNode,
  router: RouterNode,
  outcome: OutcomeNode,
};

/* ---- layout constants ----
 *  Total canvas width ~680. All nodes centered horizontally.
 *  Vertical rows: link(0) → channels(155) → router(300) → outcomes(465)
 */
const CX = 340; // canvas center-x

const nodes: Node[] = [
  {
    id: "link",
    type: "link",
    position: { x: CX - 150, y: 0 },
    draggable: false,
    selectable: false,
    data: {
      href: "go.yourcompany.com/summer-sale",
      body: "Share one link in ads, email, social, or your website.",
    },
  },
  {
    id: "channels",
    type: "channel",
    position: { x: CX - 150, y: 155 },
    draggable: false,
    selectable: false,
    data: {
      title: "Shared in places like",
      items: ["website", "email", "ads", "messages", "social", "QR codes"],
    },
  },
  {
    id: "router",
    type: "router",
    position: { x: CX - 130, y: 305 },
    draggable: false,
    selectable: false,
    data: {
      title: "Route by platform",
      body: "Rift sends each click to the best destination for that device.",
    },
  },
  {
    id: "ios",
    type: "outcome",
    position: { x: CX - 330, y: 470 },
    draggable: false,
    selectable: false,
    data: {
      eyebrow: "iPhone",
      title: "App or App Store",
      body: "Open the app when possible, then fall back to the App Store.",
      accent: "#7dd3fc",
      text: "text-[#7dd3fc]",
    },
  },
  {
    id: "android",
    type: "outcome",
    position: { x: CX - 100, y: 470 },
    draggable: false,
    selectable: false,
    data: {
      eyebrow: "Android",
      title: "App or Play Store",
      body: "Open the app when possible, then fall back to Google Play.",
      accent: "#fbbf24",
      text: "text-[#fbbf24]",
    },
  },
  {
    id: "web",
    type: "outcome",
    position: { x: CX + 130, y: 470 },
    draggable: false,
    selectable: false,
    data: {
      eyebrow: "Web",
      title: "Landing page or web URL",
      body: "Send desktop traffic to the web experience without losing metadata.",
      accent: "#f472b6",
      text: "text-[#f472b6]",
    },
  },
];

const EDGE_DEFAULTS = {
  animated: true,
  style: { strokeWidth: 1.5 },
} as const;

const edges: Edge[] = [
  {
    id: "link-channels",
    source: "link",
    target: "channels",
    ...EDGE_DEFAULTS,
    style: { ...EDGE_DEFAULTS.style, stroke: "#2dd4bf40" },
  },
  {
    id: "channels-router",
    source: "channels",
    target: "router",
    ...EDGE_DEFAULTS,
    style: { ...EDGE_DEFAULTS.style, stroke: "#2dd4bf40" },
  },
  {
    id: "router-ios",
    source: "router",
    target: "ios",
    type: "smoothstep",
    ...EDGE_DEFAULTS,
    style: { ...EDGE_DEFAULTS.style, stroke: "#7dd3fc50" },
  },
  {
    id: "router-android",
    source: "router",
    target: "android",
    type: "smoothstep",
    ...EDGE_DEFAULTS,
    style: { ...EDGE_DEFAULTS.style, stroke: "#fbbf2450" },
  },
  {
    id: "router-web",
    source: "router",
    target: "web",
    type: "smoothstep",
    ...EDGE_DEFAULTS,
    style: { ...EDGE_DEFAULTS.style, stroke: "#f472b650" },
  },
];

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
            This is the payoff of the setup flow: one public URL for people to
            click, with Rift routing it to the right place and preserving
            tracking, attribution, and machine-readable context.
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
          <div className="h-[640px] w-full">
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
              panOnDrag={false}
              zoomOnPinch={false}
              zoomOnDoubleClick={false}
              preventScrolling={false}
            >
              <Background
                gap={32}
                size={0.8}
                color="rgba(148, 163, 184, 0.04)"
              />
            </ReactFlow>
          </div>
        </div>
      </div>
    </section>
  );
}
