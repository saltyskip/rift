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

function LinkNode({ data }: NodeProps<LinkFlowNode>) {
  return (
    <div className="w-[288px] rounded-[20px] border border-[#2dd4bf]/25 bg-[linear-gradient(180deg,#10211f_0%,#0f1416_100%)] p-5 text-left shadow-[0_14px_34px_rgba(0,0,0,0.16)]">
      <Handle
        type="source"
        position={Position.Bottom}
        className="!h-3 !w-3 !opacity-0"
      />
      <p className="text-[11px] font-semibold uppercase tracking-[0.2em] text-[#2dd4bf]">Branded Link</p>
      <p className="mt-3 text-[20px] font-semibold leading-tight text-[#fafafa]">{data.href}</p>
      <p className="mt-2 text-[13px] leading-relaxed text-[#a3b8b4]">{data.body}</p>
      <div className="mt-4 flex flex-wrap gap-2 text-[11px] text-[#dff7f1]">
        <span className="rounded-full border border-[#2dd4bf]/20 bg-[#2dd4bf]/10 px-2.5 py-1">
          shared everywhere
        </span>
        <span className="rounded-full border border-[#2dd4bf]/20 bg-[#2dd4bf]/10 px-2.5 py-1">
          same URL
        </span>
      </div>
    </div>
  );
}

function ChannelNode({ data }: NodeProps<ChannelFlowNode>) {
  return (
    <div className="w-[468px] rounded-[18px] border border-[#1b2430] bg-[linear-gradient(180deg,#10141b_0%,#0d1015_100%)] p-4 text-left shadow-[0_10px_24px_rgba(0,0,0,0.12)]">
      <Handle
        type="target"
        position={Position.Top}
        className="!h-3 !w-3 !opacity-0"
      />
      <Handle
        type="source"
        position={Position.Bottom}
        className="!h-3 !w-3 !opacity-0"
      />
      <p className="text-[11px] font-semibold uppercase tracking-[0.2em] text-[#94a3b8]">{data.title}</p>
      <div className="mt-3 flex flex-wrap gap-2">
        {data.items.map((item) => (
          <span
            key={item}
            className="rounded-full border border-[#263244] bg-[#111723] px-3 py-1.5 text-[12px] text-[#dbe4ef]"
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
    <div className="w-[232px] rounded-[20px] border border-[#1f2937] bg-[linear-gradient(180deg,#12151b_0%,#0d1014_100%)] p-4 text-center shadow-[0_12px_28px_rgba(0,0,0,0.14)]">
      <Handle
        type="target"
        position={Position.Top}
        className="!h-3 !w-3 !opacity-0"
      />
      <Handle
        type="source"
        position={Position.Bottom}
        className="!h-3 !w-3 !opacity-0"
      />
      <p className="text-[11px] font-semibold uppercase tracking-[0.2em] text-[#7dd3fc]">Rift</p>
      <p className="mt-2 text-[17px] font-semibold text-[#fafafa]">{data.title}</p>
      <p className="mt-2 text-[13px] leading-relaxed text-[#9aa4b2]">{data.body}</p>
    </div>
  );
}

function OutcomeNode({ data }: NodeProps<OutcomeFlowNode>) {
  return (
    <div className="w-[214px] rounded-[20px] border border-[#1f2937] bg-[linear-gradient(180deg,#121316_0%,#0d0f13_100%)] p-4 text-left shadow-[0_12px_28px_rgba(0,0,0,0.14)]">
      <Handle
        type="target"
        position={Position.Top}
        className="!h-3 !w-3 !opacity-0"
      />
      <p className={`text-[11px] font-semibold uppercase tracking-[0.2em] ${data.text}`}>
        {data.eyebrow}
      </p>
      <p className="mt-2 text-[16px] font-semibold text-[#fafafa]">{data.title}</p>
      <p className="mt-2 text-[13px] leading-relaxed text-[#9aa0aa]">{data.body}</p>
    </div>
  );
}

const nodeTypes = {
  link: LinkNode,
  channel: ChannelNode,
  router: RouterNode,
  outcome: OutcomeNode,
};

const nodes: Node[] = [
  {
    id: "link",
    type: "link",
    position: { x: 185, y: 18 },
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
    position: { x: 96, y: 178 },
    draggable: false,
    selectable: false,
    data: {
      title: "Shared in places like",
      items: ["website", "email", "Facebook ads", "messages", "social", "QR codes"],
    },
  },
  {
    id: "router",
    type: "router",
    position: { x: 214, y: 314 },
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
    position: { x: 18, y: 470 },
    draggable: false,
    selectable: false,
    data: {
      eyebrow: "iPhone",
      title: "App or App Store",
      body: "Open the app when possible, then fall back to the App Store.",
      accent: "!bg-[#7dd3fc]",
      text: "text-[#7dd3fc]",
    },
  },
  {
    id: "android",
    type: "outcome",
    position: { x: 260, y: 470 },
    draggable: false,
    selectable: false,
    data: {
      eyebrow: "Android",
      title: "App or Play Store",
      body: "Open the app when possible, then fall back to Google Play.",
      accent: "!bg-[#fbbf24]",
      text: "text-[#fbbf24]",
    },
  },
  {
    id: "web",
    type: "outcome",
    position: { x: 502, y: 470 },
    draggable: false,
    selectable: false,
    data: {
      eyebrow: "Web",
      title: "Landing page or web URL",
      body: "Send desktop traffic to the web experience without losing metadata.",
      accent: "!bg-[#f472b6]",
      text: "text-[#f472b6]",
    },
  },
];

const edges: Edge[] = [
  {
    id: "link-channels",
    source: "link",
    target: "channels",
    markerEnd: { type: MarkerType.ArrowClosed, color: "#2dd4bf" },
    style: { stroke: "#2dd4bf", strokeWidth: 2.2 },
    animated: true,
  },
  {
    id: "channels-router",
    source: "channels",
    target: "router",
    markerEnd: { type: MarkerType.ArrowClosed, color: "#2dd4bf" },
    style: { stroke: "#2dd4bf", strokeWidth: 2.2 },
    animated: true,
  },
  {
    id: "router-ios",
    source: "router",
    target: "ios",
    markerEnd: { type: MarkerType.ArrowClosed, color: "#7dd3fc" },
    style: { stroke: "#7dd3fc", strokeWidth: 2 },
    type: "smoothstep",
    animated: true,
  },
  {
    id: "router-android",
    source: "router",
    target: "android",
    markerEnd: { type: MarkerType.ArrowClosed, color: "#fbbf24" },
    style: { stroke: "#fbbf24", strokeWidth: 2 },
    type: "smoothstep",
    animated: true,
  },
  {
    id: "router-web",
    source: "router",
    target: "web",
    markerEnd: { type: MarkerType.ArrowClosed, color: "#f472b6" },
    style: { stroke: "#f472b6", strokeWidth: 2 },
    type: "smoothstep",
    animated: true,
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

      <div className="rounded-[22px] border border-[#1a1c21] bg-[linear-gradient(180deg,#0f1217_0%,#0b0d11_100%)] p-3 md:p-4">
        <div className="overflow-hidden rounded-[16px] border border-[#141820] bg-[radial-gradient(circle_at_top,#101a1f_0%,#0d1015_48%,#0b0d11_100%)]">
          <div className="h-[660px] w-full">
            <ReactFlow
              nodes={nodes}
              edges={edges}
              nodeTypes={nodeTypes}
              fitView
              fitViewOptions={{ padding: 0.045 }}
              defaultViewport={{ x: 0, y: 0, zoom: 1 }}
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
