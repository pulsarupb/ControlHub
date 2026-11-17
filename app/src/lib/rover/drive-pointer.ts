// Map a pointer position to raw drive coordinates for an element: x is positive to the
// right, y is positive upward, both 1 at the element edge. Values are not clamped here;
// the rover control clamps each axis, and joystick pads clamp to a circle themselves.
export function normalizedPointer(el: HTMLElement, event: PointerEvent): { x: number; y: number } {
  const rect = el.getBoundingClientRect();
  const halfWidth = rect.width * 0.5;
  const halfHeight = rect.height * 0.5;
  const centerX = rect.left + halfWidth;
  const centerY = rect.top + halfHeight;
  return {
    x: (event.clientX - centerX) / halfWidth,
    y: -(event.clientY - centerY) / halfHeight,
  };
}
