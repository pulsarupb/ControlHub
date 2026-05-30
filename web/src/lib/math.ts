export function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}

export function radiansToDegrees(value: number): number {
  return (value * 180) / Math.PI;
}
