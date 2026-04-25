import { Composition } from "remotion";
import { ReadmeDemo } from "./ReadmeDemo";
import { HeroStill, HomeStill, PlayerStill } from "./StillFrames";

const width = 1600;
const height = 1000;
const fps = 30;

export function RemotionRoot() {
  return (
    <>
      <Composition
        id="ReadmeDemo"
        component={ReadmeDemo}
        durationInFrames={720}
        fps={fps}
        width={width}
        height={height}
      />
      <Composition
        id="HeroStill"
        component={HeroStill}
        durationInFrames={1}
        fps={fps}
        width={width}
        height={height}
      />
      <Composition
        id="HomeStill"
        component={HomeStill}
        durationInFrames={1}
        fps={fps}
        width={width}
        height={height}
      />
      <Composition
        id="PlayerStill"
        component={PlayerStill}
        durationInFrames={1}
        fps={fps}
        width={width}
        height={height}
      />
    </>
  );
}
