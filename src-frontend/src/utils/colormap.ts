export interface RGB {
  r: number;
  g: number;
  b: number;
}

/**
 * Maps dB magnitude to RGB color (Viridis-like colormap)
 * @param db - Magnitude in dB (typically -80 to 0)
 * @returns RGB color
 */
export function magnitudeToColor(db: number): RGB {
  // Map -80dB to 0dB → 0 to 1 range
  const normalized = (db + 80) / 80;
  const clamped = Math.max(0, Math.min(1, normalized));

  // Simple linear interpolation (blue → cyan → green → yellow → red)
  if (clamped < 0.25) {
    const t = clamped * 4;
    return { r: 0, g: 0, b: Math.floor(255 * (1 - t)) };
  } else if (clamped < 0.5) {
    const t = (clamped - 0.25) * 4;
    return { r: 0, g: Math.floor(255 * t), b: 255 };
  } else if (clamped < 0.75) {
    const t = (clamped - 0.5) * 4;
    return { r: Math.floor(255 * t), g: 255, b: Math.floor(255 * (1 - t)) };
  } else {
    const t = (clamped - 0.75) * 4;
    return { r: 255, g: Math.floor(255 * (1 - t)), b: 0 };
  }
}
