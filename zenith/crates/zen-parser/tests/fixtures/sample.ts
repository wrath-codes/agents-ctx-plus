/**
 * Maximum number of retries for operations.
 */
export const MAX_RETRIES: number = 3;

const INTERNAL_TIMEOUT: number = 5000;

/**
 * A result type that can be either success or failure.
 * @template T The success value type
 */
export type Result<T> = { ok: true; value: T } | { ok: false; error: Error };

type InternalState = "idle" | "running" | "stopped";

/**
 * Handler interface for processing items.
 * @template T The input type
 */
export interface Handler<T> {
    handle(input: T): Promise<void>;
    readonly name: string;
}

interface PrivateConfig {
    timeout: number;
    retries: number;
}

/**
 * Event emitter class with typed events.
 */
export default class EventEmitter {
    private listeners: Map<string, Function[]> = new Map();

    /**
     * Register an event listener.
     * @param event The event name
     * @param callback The callback function
     */
    on(event: string, callback: Function): void {
        // register listener
    }

    emit(event: string, data: unknown): void {
        // emit event
    }

    private cleanup(): void {
        // internal cleanup
    }
}

export class HttpError extends Error {
    constructor(public status: number, message: string) {
        super(message);
    }
}

/**
 * Process a list of items using a handler.
 * @param items The items to process
 * @param handler The handler to use
 * @returns A result containing the processed items
 * @throws Error if handler fails
 */
export async function processItems<T>(
    items: T[],
    handler: Handler<T>,
): Promise<Result<T[]>> {
    return { ok: true, value: items };
}

function internalHelper(x: number): number {
    return x * 2;
}

/**
 * Fetch data from a URL.
 * @param url The URL to fetch from
 * @returns The fetch response
 */
export const fetchData = async (url: string): Promise<Response> => {
    return fetch(url);
};

export const add = (a: number, b: number): number => a + b;

const multiply = (a: number, b: number): number => a * b;

/**
 * Color enumeration.
 */
export enum Color {
    Red,
    Green,
    Blue,
}

enum InternalStatus {
    Active,
    Inactive,
}

export abstract class Shape {
    abstract area(): number;

    perimeter(): number {
        return 0;
    }
}

// ── Namespace ──────────────────────────────────────────────────────

/**
 * String validation utilities.
 */
export namespace Validators {
    export function isEmail(s: string): boolean {
        return s.includes("@");
    }

    export interface Rule {
        check(value: string): boolean;
    }

    export const MAX_LENGTH: number = 255;
}

namespace InternalUtils {
    export function hash(s: string): number {
        return s.length;
    }
}

// ── Ambient declarations (.d.ts style) ─────────────────────────────

declare function fetchExternal(url: string): Promise<Response>;

declare const API_VERSION: string;

declare class ExternalLib {
    run(): void;
}

declare module "my-module" {
    export function init(): void;
}

// ── Function overloads ─────────────────────────────────────────────

/**
 * Greet a person or convert age to greeting.
 */
function greet(name: string): string;
function greet(age: number): string;
function greet(value: any): string {
    return String(value);
}

// ── Variable declarations (var/let) ────────────────────────────────

let counter: number = 0;
var legacyFlag: boolean = true;

// ── Enum with string values ────────────────────────────────────────

export enum Direction {
    Up = "UP",
    Down = "DOWN",
    Left = "LEFT",
    Right = "RIGHT",
}
