import { ROVER_LENGTH_M, ROVER_WIDTH_M } from "$lib/constants";
import { radiansToDegrees } from "$lib/math";
import type { MapSize, PathPoint, Pose } from "$lib/types";

export function mapScale(points: PathPoint[], mapSize: MapSize): number {
  const footprintRadius = Math.hypot(ROVER_LENGTH_M, ROVER_WIDTH_M) * 0.5;
  const maxDistance = points.reduce(
    (max, point) => Math.max(max, Math.abs(point.x_m), Math.abs(point.y_m)),
    footprintRadius,
  );

  return (Math.min(mapSize.width, mapSize.height) * 0.38) / Math.max(1, maxDistance);
}

export function buildPathPoints(points: PathPoint[], mapSize: MapSize): string {
  if (!points.length) return "";

  const scale = mapScale(points, mapSize);
  return points
    .map((point) => `${mapSize.width / 2 + point.x_m * scale},${mapSize.height / 2 - point.y_m * scale}`)
    .join(" ");
}

export function robotPoseTransform(point: Pose, mapSize: MapSize, scale: number): string {
  const x = mapSize.width / 2 + point.x_m * scale;
  const y = mapSize.height / 2 - point.y_m * scale;
  const deg = -radiansToDegrees(point.heading_rad);

  return `translate(${x} ${y}) rotate(${deg})`;
}

export function roverBodyPath(lengthPx: number, widthPx: number): string {
  return `M ${lengthPx / 2} 0 L ${lengthPx * 0.25} ${-widthPx / 2} L ${-lengthPx / 2} ${-widthPx / 2} L ${-lengthPx / 2} ${widthPx / 2} L ${lengthPx * 0.25} ${widthPx / 2} Z`;
}
