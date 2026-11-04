import { useState } from "react";
import { Canvas, CanvasText, Rectangle } from "ink";
import { defineOverlay } from "@ink/overlays";

function DemoOverlay({ message }: { message: string }) {
  const [taps, setTaps] = useState(0);
  return (
    <Canvas width={360} height={150}>
      <Rectangle x={0} y={0} width={360} height={150} fill="#ffffff" />
      <CanvasText x={12} y={8} width={336} height={30} size={24} colour="#000000">
        Ink overlay
      </CanvasText>
      <CanvasText x={12} y={42} width={336} height={24} size={18} colour="#000000">
        {message}
      </CanvasText>
      <CanvasText x={12} y={72} width={336} height={24} size={20} colour="#000000">
        {`Taps: ${taps}`}
      </CanvasText>
      <Rectangle x={12} y={105} width={158} height={33} fill="#000000" onPress={() => setTaps(value => value + 1)} />
      <CanvasText x={12} y={108} width={158} height={28} size={20} colour="#ffffff" align="center">
        Tap
      </CanvasText>
      <Rectangle x={190} y={105} width={158} height={33} fill="#000000" onPress={() => { void demoOverlay.hide(); }} />
      <CanvasText x={190} y={108} width={158} height={28} size={20} colour="#ffffff" align="center">
        Close
      </CanvasText>
    </Canvas>
  );
}

export const demoOverlay = defineOverlay({ id: "template.demo", component: DemoOverlay });
