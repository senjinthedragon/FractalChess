/* tslint:disable */
/* eslint-disable */

export function config_add_piece(r: number, g: number, b: number, offsets: Int32Array, enemy_mask: number): void;

export function config_reset(): void;

export function pixel_ptr(): number;

export function placed_count(): number;

export function redraw(): void;

export function set_view(cx: number, cy: number, radius: number): void;

export function simulate_batch(target_radius: number, max_turns: number): number;

export function start(tex_size: number): void;

export function target_reached(target_radius: number): boolean;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly config_add_piece: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
    readonly config_reset: () => void;
    readonly pixel_ptr: () => number;
    readonly placed_count: () => number;
    readonly redraw: () => void;
    readonly set_view: (a: number, b: number, c: number) => void;
    readonly simulate_batch: (a: number, b: number) => number;
    readonly start: (a: number) => void;
    readonly target_reached: (a: number) => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
