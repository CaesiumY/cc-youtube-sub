import { AbsoluteFill } from "remotion";
import { AppPreview } from "./AppPreview";

export function HeroStill() {
  return (
    <AbsoluteFill style={{ background: "#030712" }}>
      <AppPreview mode="player" progress={0.72} showOriginal />
    </AbsoluteFill>
  );
}

export function HomeStill() {
  return (
    <AbsoluteFill style={{ background: "#030712" }}>
      <AppPreview mode="home" progress={0} />
    </AbsoluteFill>
  );
}

export function PlayerStill() {
  return (
    <AbsoluteFill style={{ background: "#030712" }}>
      <AppPreview mode="player" progress={0.58} showOriginal />
    </AbsoluteFill>
  );
}
