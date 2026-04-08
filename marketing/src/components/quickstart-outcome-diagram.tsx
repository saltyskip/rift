"use client";

import {
  Background,
  Handle,
  MarkerType,
  type Node as FlowNode,
  Position,
  ReactFlow,
  type Edge,
  type Node,
  type NodeProps,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";

type LinkNodeData = { title: string; href: string; body: string };
type RouterNodeData = { title: string; body: string };
type OutcomeNodeData = {
  eyebrow: string;
  title: string;
  body: string;
  accent: string;
  text: string;
};

type LinkFlowNode = FlowNode<LinkNodeData, "link">;
type RouterFlowNode = FlowNode<RouterNodeData, "router">;
type OutcomeFlowNode = FlowNode<OutcomeNodeData, "outcome">;

function LinkNode({ data }: NodeProps<LinkFlowNode>) {
  return (
    <div className="w-[320px] rounded-[24px] border border-[#2dd4bf]/25 bg-[linear-gradient(180deg,#10211f_0%,#0f1416_100%)] p-5 text-left shadow-[0_18px_60px_rgba(0,0,0,0.28)]">
      <Handle type="source" position={Position.Bottom} className="!h-3 !w-3 !border-2 !border-[#0f1416] !bg-[#2dd4bf]" />
      <p className="text-[11px] font-semibold uppercase tracking-[0.2em] text-[#2dd4bf]">Branded Link</p>
      <p className="mt-3 text-[22px] font-semibold leading-tight text-[#fafafa]">{data.href}</p>
      <p className="mt-2 text-[14px] leading-relaxed text-[#a3b8b4]">{data.body}</p>
      <div className="mt-4 flex flex-wrap gap-2 text-[11px] text-[#dff7f1]">
        <span className="rounded-full border border-[#2dd4bf]/20 bg-[#2dd4bf]/10 px-2.5 py-1">shared everywhere</span>
        <span className="rounded-full border border-[#2dd4bf]/20 bg-[#2dd4bf]/10 px-2.5 py-1">same URL</span>
      </div>
    </div>
  );
}

function RouterNode({ data }: NodeProps<RouterFlowNode>) {
  return (
    <div className="w-[220px] rounded-[22px] border border-[#1f2937] bg-[linear-gradient(180deg,#12151b_0%,#0d1014_100%)] p-4 text-center shadow-[0_14px_40px_rgba(0,0,0,0.24)]">
      <Handle type="target" position={Position.Top} className="!h-3 !w-3 !border-2 !border-[#0d1014] !bg-[#2dd4bf]" />
      <Handle type="source" position={Position.Bottom} className="!h-3 !w-3 !border-2 !border-[#0d1014] !bg-[#2dd4bf]" />
      <p className="text-[11px] font-semibold uppercase tracking-[0.2em] text-[#7dd3fc]">Rift</p>
      <p className="mt-2 text-[18px] font-semibold text-[#fafafa]">{data.title}</p>
      <p className="mt-2 text-[13px] leading-relaxed text-[#9aa4b2]">{data.body}</p>
    </div>
  );
}

function OutcomeNode({
  data,
}: NodeProps<OutcomeFlowNode>) {
  return (
    <div className="w-[240px] rounded-[22px] border border-[#1f2937] bg-[linear-gradient(180deg,#121316_0%,#0d0f13_100%)] p-4 text-left shadow-[0_14px_40px_rgba(0,0,0,0.2)]">
      <Handle type="target" position={Position.Top} className={`!h-3 !w-3 !border-2 !border-[#0d0f13] ${data.accent}`} />
      <p className={`text-[11px] font-semibold uppercase tracking-[0.2em] ${data.text}`}>{data.eyebrow}</p>
      <p className="mt-2 text-[17px] font-semibold text-[#fafafa]">{data.title}</p>
      <p className="mt-2 text-[13px] leading-relaxed text-[#9aa0aa]">{data.body}</p>
    </div>
  );
}

const nodeTypes = {
  link: LinkNode,
  router: RouterNode,
  outcome: OutcomeNode,
};

const nodes: Node[] = [
  {
    id: "link",
    type: "link",
    position: { x: 180, y: 20 },
    draggable: false,
    selectable: false,
    data: {
      title: "Branded Link",
      href: "go.yourcompany.com/summer-sale",
      body: "One link you can drop into ads, email, social, or your website.",
    },
  },
  {
    id: "router",
    type: "router",
    position: { x: 230, y: 230 },
    draggable: false,
    selectable: false,
    data: {
      title: "Route by platform",
      body: "Rift decides where to send people based on device, install state, and your link config.",
    },
  },
  {
    id: "ios",
    type: "outcome",
    position: { x: 20, y: 430 },
    draggable: false,
    selectable: false,
    data: {
      eyebrow: "iPhone",
      title: "Open the app",
      body: "Universal Links first, then App Store fallback if the app is not installed.",
      accent: "!bg-[#7dd3fc]",
      text: "text-[#7dd3fc]",
    },
  },
  {
    id: "android",
    type: "outcome",
    position: { x: 270, y: 430 },
    draggable: false,
    selectable: false,
    data: {
      eyebrow: "Android",
      title: "Open the app",
      body: "App Links first, then Play Store fallback while keeping the same shareable URL.",
      accent: "!bg-[#fbbf24]",
      text: "text-[#fbbf24]",
    },
  },
  {
    id: "web",
    type: "outcome",
    position: { x: 520, y: 430 },
    draggable: false,
    selectable: false,
    data: {
      eyebrow: "Web",
      title: "Show the web destination",
      body: "Desktop and unsupported cases land on the web experience without losing metadata.",
      accent: "!bg-[#f472b6]",
      text: "text-[#f472b6]",
    },
  },
];

const edges: Edge[] = [
  {
    id: "link-router",
    source: "link",
    target: "router",
    markerEnd: { type: MarkerType.ArrowClosed, color: "#2dd4bf" },
    style: { stroke: "#2dd4bf", strokeWidth: 2.2 },
  },
  {
    id: "router-ios",
    source: "router",
    target: "ios",
    markerEnd: { type: MarkerType.ArrowClosed, color: "#7dd3fc" },
    style: { stroke: "#7dd3fc", strokeWidth: 2 },
    type: "smoothstep",
  },
  {
    id: "router-android",
    source: "router",
    target: "android",
    markerEnd: { type: MarkerType.ArrowClosed, color: "#fbbf24" },
    style: { stroke: "#fbbf24", strokeWidth: 2 },
    type: "smoothstep",
  },
  {
    id: "router-web",
    source: "router",
    target: "web",
    markerEnd: { type: MarkerType.ArrowClosed, color: "#f472b6" },
    style: { stroke: "#f472b6", strokeWidth: 2 },
    type: "smoothstep",
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
          <h2 className="mt-3 text-2xl font-bold text-[#fafafa]">One branded link, routed everywhere</h2>
          <p className="mt-2 max-w-2xl text-[14px] leading-relaxed text-[#8f96a3]">
            This is the payoff of the setup flow: one public URL for people to click, with Rift routing it
            to the right place and preserving tracking, attribution, and machine-readable context.
          </p>
        </div>
        <div className="flex flex-wrap gap-2 text-[12px] text-[#d4d4d8]">
          <span className="rounded-full border border-[#2dd4bf]/20 bg-[#2dd4bf]/10 px-3 py-1">click tracked</span>
          <span className="rounded-full border border-[#60a5fa]/20 bg-[#60a5fa]/10 px-3 py-1">attribution ready</span>
          <span className="rounded-full border border-[#f59e0b]/20 bg-[#f59e0b]/10 px-3 py-1">agent-readable</span>
        </div>
      </div>

      <div className="rounded-[30px] border border-[#1e1e22] bg-[radial-gradient(circle_at_top,#12201f_0%,#0c0d10_44%,#0b0c0f_100%)] p-3 md:p-5">
        <div className="overflow-hidden rounded-[24px] border border-[#16191f] bg-[linear-gradient(180deg,#0f1318_0%,#0b0d11_100%)]">
          <div className="h-[700px] w-full">
            <ReactFlow
              nodes={nodes}
              edges={edges}
              nodeTypes={nodeTypes}
              fitView
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
              <Background gap={24} size={1} color="rgba(148, 163, 184, 0.08)" />
            </ReactFlow>
          </div>
        </div>
      </div>
    </section>
  );
}
