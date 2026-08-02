import type { Mountain, MountainNode } from '../ts/types.js';

function ordinalToLatex(ord: number[]): string {
  let end = ord.length;
  while (end > 0 && ord[end - 1] === 0) end--;
  if (end === 0) return '0';
  const parts: string[] = [];
  let first = true;
  for (let i = end - 1; i >= 0; i--) {
    const c = ord[i];
    if (c === 0) continue;
    if (!first) parts.push('+');
    first = false;
    if (i === 0) {
      parts.push(String(c));
    } else if (i === 1) {
      parts.push(c === 1 ? 'ω' : 'ω' + c);
    } else {
      parts.push('ω<tspan baseline-shift="super" font-size="0.65em">' + i + '</tspan>' + (c > 1 ? c : ''));
    }
  }
  return parts.join('');
}

function separatorLineCount(rowA: number[], rowB: number[]): number {
  const maxLen = Math.max(rowA.length, rowB.length);
  for (let i = maxLen - 1; i >= 0; i--) {
    const a = i < rowA.length ? rowA[i] : 0;
    const b = i < rowB.length ? rowB[i] : 0;
    if (a !== b) return i === 0 ? 0 : i;
  }
  return 0;
}

// JetBrains Mono is monospace with a 0.6em advance; labels render at
// font-size 14 with superscripts at 0.65em.
const MONO_W = 14 * 0.6; // 8.4 px per baseline character
const MONO_SUP_W = 14 * 0.65 * 0.6; // 5.46 px per superscript character

/** Exact rendered width (px) of an ordinal row label. */
function ordinalLabelWidth(ord: number[]): number {
  let end = ord.length;
  while (end > 0 && ord[end - 1] === 0) end--;
  let w = 0;
  let first = true;
  for (let i = end - 1; i >= 0; i--) {
    const c = ord[i];
    if (c === 0) continue;
    if (!first) w += MONO_W; // '+'
    first = false;
    if (i === 0) {
      w += MONO_W * String(c).length;
    } else if (i === 1) {
      // c === 1 renders as just "ω"
      w += MONO_W * (c === 1 ? 1 : 1 + String(c).length);
    } else {
      // "ω" + superscript exponent + (coefficient when c > 1)
      w += MONO_W + MONO_SUP_W * String(i).length + (c > 1 ? MONO_W * String(c).length : 0);
    }
  }
  return w;
}

export function renderMountain0Y(mountain: Mountain): string {
  if (!mountain.length) return '';
  let layers = mountain.length;
  if (layers > 1 && mountain[layers - 1].every((n: MountainNode) => n.value === 1)) layers--;
  const cols = mountain[0].length;
  const colLabelW = 20;
  const gapX = 50;
  const gapY = 55;
  const padX = 25 + colLabelW;
  const padY = 30;
  const svgW = Math.max(cols * gapX + padX * 2, 200);
  const svgH = layers * gapY + padY * 2;

  let svg = `<svg width="${svgW}" height="${svgH}" xmlns="http://www.w3.org/2000/svg">`;
  const cx = (col: number) => col * gapX + padX;
  const cy = (layer: number) => (layers - 1 - layer) * gapY + padY;
  const off = 9;
  const cyA = (layer: number) => cy(layer) - off;
  const cyB = (layer: number) => cy(layer) + off;

  for (let layer = 1; layer < layers; layer++) {
    for (let col = 0; col < cols; col++) {
      const belowNode = mountain[layer - 1][col];
      svg += `<line x1="${cx(col)}" y1="${cyB(layer)}" x2="${cx(col)}" y2="${cyA(layer - 1)}" stroke="var(--muted)" stroke-width="1.5" stroke-linecap="round"/>`;
      if (belowNode.parent > 0) {
        const pcol = col - belowNode.parent;
        if (pcol >= 0) {
          svg += `<line x1="${cx(col)}" y1="${cyB(layer)}" x2="${cx(pcol)}" y2="${cyA(layer - 1)}" stroke="var(--muted)" stroke-width="1.5" stroke-linecap="round"/>`;
        }
      }
    }
  }

  svg += `<g font-size="16" fill="var(--muted)" text-anchor="end">`;
  svg += `<text x="${padX - colLabelW + 4}" y="${cy(layers - 1) - 16}" dominant-baseline="middle" font-size="13" fill="var(--label)">Row</text>`;
  for (let layer = 0; layer < layers; layer++) {
    svg += `<text x="${padX - colLabelW + 4}" y="${cy(layer) + 1}" dominant-baseline="middle">${layer}</text>`;
  }
  svg += `</g>`;

  for (let layer = 0; layer < layers; layer++) {
    for (let col = 0; col < cols; col++) {
      const node = mountain[layer][col];
      const x = cx(col);
      const y = cy(layer);
      svg += `<text x="${x}" y="${y + 1}" text-anchor="middle" dominant-baseline="middle" font-size="15" fill="var(--fg)" font-weight="${layer === 0 ? 'bold' : 'normal'}">${node.value}</text>`;
    }
  }

  svg += '</svg>';
  return svg;
}

export function renderMountainWY(mountain: Mountain, rowLabels: number[][]): string {
  if (!mountain.length || !mountain[0].length) return '';
  const layers = mountain.length;
  const cols = mountain[0].length;
  const gapX = 50;
  const gapY = 55;
  const maxLabelW = rowLabels.reduce((m, r) => Math.max(m, ordinalLabelWidth(r)), 0);
  const padX = Math.max(65, maxLabelW + 25);
  const padY = 30;
  const extraGap = 30;

  const layerShift: number[] = new Array(layers).fill(0);
  let totalShift = 0;
  for (let k = 1; k < layers; k++) {
    const prevRow = k - 1 < rowLabels.length ? rowLabels[k - 1] : [k - 1];
    const curRow = k < rowLabels.length ? rowLabels[k] : [k];
    if (separatorLineCount(prevRow, curRow) > 0) totalShift += extraGap;
    layerShift[k] = totalShift;
  }

  const svgW = Math.max(cols * gapX + padX * 2, 200);
  const svgH = layers * gapY + padY * 2 + totalShift;

  let svg = `<svg width="${svgW}" height="${svgH}" xmlns="http://www.w3.org/2000/svg">`;
  const cx = (col: number) => col * gapX + padX;
  const cy = (layer: number) => (layers - 1 - layer) * gapY + padY + totalShift - layerShift[layer];
  const off = 9;
  const cyA = (layer: number) => cy(layer) - off;
  const cyB = (layer: number) => cy(layer) + off;
  const normalYDisp = gapY - 2 * off;

  const lastRow: number[] = new Array(cols).fill(-1);

  svg += `<g stroke="var(--border)" fill="none">`;
  for (let k = 1; k < layers; k++) {
    const prevRow = k - 1 < rowLabels.length ? rowLabels[k - 1] : [k - 1];
    const curRow = k < rowLabels.length ? rowLabels[k] : [k];
    const nLines = separatorLineCount(prevRow, curRow);
    if (nLines === 0) continue;
    const sepExt = 12;
    const lineSpacing = 4;
    const gapTop = cyB(k) + normalYDisp;
    const gapBottom = cyA(k - 1);
    const gapMid = (gapTop + gapBottom) / 2;
    const yStart = gapMid - ((nLines - 1) * lineSpacing) / 2;
    for (let n = 0; n < nLines; n++) {
      const y = yStart + n * lineSpacing;
      svg += `<line x1="${padX - sepExt}" y1="${y}" x2="${padX + (cols - 1) * gapX + sepExt}" y2="${y}" stroke-width="1.5"/>`;
    }
  }
  svg += `</g>`;

  for (let layer = 0; layer < layers; layer++) {
    for (let col = 0; col < cols; col++) {
      const node = mountain[layer][col];
      if (node.value < 0) continue;

      if (lastRow[col] >= 0) {
        svg += `<line x1="${cx(col)}" y1="${cyB(layer)}" x2="${cx(col)}" y2="${cyA(lastRow[col])}" stroke="var(--fg)" stroke-width="1.5" stroke-linecap="round"/>`;
      }

      const parentCol = node.parentCol ?? -1;
      if (parentCol >= 0 && parentCol < cols && layer > 0) {
        const diagEndY = cyB(layer) + normalYDisp;
        svg += `<line x1="${cx(col)}" y1="${cyB(layer)}" x2="${cx(parentCol)}" y2="${diagEndY}" stroke="var(--fg)" stroke-width="1.5" stroke-linecap="round"/>`;
        if (Math.abs(diagEndY - cyA(layer - 1)) > 1) {
          svg += `<line x1="${cx(parentCol)}" y1="${diagEndY}" x2="${cx(parentCol)}" y2="${cyA(layer - 1)}" stroke="var(--fg)" stroke-width="1.5" stroke-linecap="round"/>`;
        }
        if (lastRow[parentCol] >= 0 && lastRow[parentCol] < layer - 1) {
          svg += `<line x1="${cx(parentCol)}" y1="${cyA(layer - 1)}" x2="${cx(parentCol)}" y2="${cyA(lastRow[parentCol])}" stroke="var(--fg)" stroke-width="1.5" stroke-linecap="round"/>`;
        }
      }

      lastRow[col] = layer;
    }
  }

  svg += `<g font-size="14" fill="var(--muted)" text-anchor="end">`;
  svg += `<text x="${padX - 20}" y="${cy(layers - 1) - 16}" dominant-baseline="middle" font-size="12" fill="var(--label)">Row</text>`;
  for (let layer = 0; layer < layers; layer++) {
    const ord = layer < rowLabels.length ? rowLabels[layer] : [layer];
    const label = ordinalToLatex(ord);
    svg += `<text x="${padX - 20}" y="${cy(layer) + 1}" dominant-baseline="middle">${label}</text>`;
  }
  svg += `</g>`;

  for (let layer = 0; layer < layers; layer++) {
    for (let col = 0; col < cols; col++) {
      const node = mountain[layer][col];
      if (node.value < 0) continue;
      svg += `<text x="${cx(col)}" y="${cy(layer) + 1}" text-anchor="middle" dominant-baseline="middle" font-size="15" fill="var(--fg)" font-weight="${layer === 0 ? 'bold' : 'normal'}">${node.value}</text>`;
    }
  }

  svg += '</svg>';
  return svg;
}

export function renderMountain1Y(mountain: Mountain, rowLabels: number[][]): string {
  if (!mountain.length) return '';
  const layers = mountain.length;
  const cols = mountain.reduce((max, layer) => Math.max(max, layer.length), 0);
  if (cols === 0) return '';
  const gapX = 50;
  const gapY = 55;
  const maxLabelW = rowLabels.reduce((m, r) => Math.max(m, ordinalLabelWidth(r)), 0);
  const padX = Math.max(65, maxLabelW + 25);
  const padY = 30;
  const svgW = Math.max(cols * gapX + padX * 2, 200);
  const svgH = layers * gapY + padY * 2;

  let svg = `<svg width="${svgW}" height="${svgH}" xmlns="http://www.w3.org/2000/svg">`;
  const cx = (col: number) => col * gapX + padX;
  const cy = (layer: number) => (layers - 1 - layer) * gapY + padY;
  const off = 9;
  const cyA = (layer: number) => cy(layer) - off;
  const cyB = (layer: number) => cy(layer) + off;

  const isNewExtraction: boolean[] = new Array(layers).fill(false);
  for (let i = 1; i < layers; i++) {
    const row = rowLabels[i] || [i];
    isNewExtraction[i] = row.length > 0 && row[0] === 0;
  }

  svg += `<g stroke="var(--border)" fill="none">`;
  for (let layer = 0; layer < layers; layer++) {
    if (!isNewExtraction[layer]) continue;
    const sepY = (cy(layer) + cy(layer - 1)) / 2;
    const sepExt = 12;
    svg += `<line x1="${padX - sepExt}" y1="${sepY}" x2="${padX + (cols - 1) * gapX + sepExt}" y2="${sepY}" stroke-width="1.5"/>`;
  }
  svg += `</g>`;

  const lastRow: number[] = new Array(cols).fill(-1);

  for (let layer = 0; layer < layers; layer++) {
    for (let col = 0; col < cols; col++) {
      const node = col < mountain[layer].length ? mountain[layer][col] : null;
      if (!node || node.value < 0) continue;

      if (lastRow[col] >= 0) {
        const strokeStyle = isNewExtraction[layer] ? 'stroke="var(--muted)" stroke-dasharray="4,3"' : 'stroke="var(--fg)"';
        svg += `<line x1="${cx(col)}" y1="${cyB(layer)}" x2="${cx(col)}" y2="${cyA(lastRow[col])}" ${strokeStyle} stroke-width="1.5" stroke-linecap="round"/>`;
      }

      const parentCol = node.parentCol ?? -1;
      if (parentCol >= 0 && parentCol < cols && layer > 0 && !isNewExtraction[layer]) {
        svg += `<line x1="${cx(col)}" y1="${cyB(layer)}" x2="${cx(parentCol)}" y2="${cyA(layer - 1)}" stroke="var(--fg)" stroke-width="1.5" stroke-linecap="round"/>`;
        if (lastRow[parentCol] >= 0 && lastRow[parentCol] < layer - 1) {
          svg += `<line x1="${cx(parentCol)}" y1="${cyA(layer - 1)}" x2="${cx(parentCol)}" y2="${cyA(lastRow[parentCol])}" stroke="var(--fg)" stroke-width="1.5" stroke-linecap="round"/>`;
        }
      }

      lastRow[col] = layer;
    }
  }

  svg += `<g font-size="14" fill="var(--muted)" text-anchor="end">`;
  svg += `<text x="${padX - 20}" y="${cy(layers - 1) - 16}" dominant-baseline="middle" font-size="12" fill="var(--label)">Row</text>`;
  for (let layer = 0; layer < layers; layer++) {
    const ord = layer < rowLabels.length ? rowLabels[layer] : [layer];
    const label = ordinalToLatex(ord);
    svg += `<text x="${padX - 20}" y="${cy(layer) + 1}" dominant-baseline="middle">${label}</text>`;
  }
  svg += `</g>`;

  for (let layer = 0; layer < layers; layer++) {
    for (let col = 0; col < cols; col++) {
      const node = col < mountain[layer].length ? mountain[layer][col] : null;
      if (!node || node.value < 0) continue;
      svg += `<text x="${cx(col)}" y="${cy(layer) + 1}" text-anchor="middle" dominant-baseline="middle" font-size="15" fill="var(--fg)" font-weight="${layer === 0 ? 'bold' : 'normal'}">${node.value}</text>`;
    }
  }

  svg += '</svg>';
  return svg;
}
