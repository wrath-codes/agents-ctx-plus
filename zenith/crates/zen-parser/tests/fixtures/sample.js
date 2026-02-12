// ── Regular functions ──────────────────────────────────────────────

/**
 * Calculate the sum of numbers.
 * @param {number[]} numbers - Array of numbers.
 * @returns {number} The sum.
 */
function sum(numbers) {
    return numbers.reduce((a, b) => a + b, 0);
}

function internalHelper() {
    return 42;
}

// ── Async functions ───────────────────────────────────────────────

async function fetchData(url) {
    const response = await fetch(url);
    return response.json();
}

// ── Generator functions ───────────────────────────────────────────

/**
 * Generate sequential numbers.
 * @param {number} max - Upper bound.
 * @yields {number} Sequential numbers.
 */
function* generateNumbers(max) {
    for (let i = 0; i < max; i++) {
        yield i;
    }
}

async function* asyncStream() {
    yield 1;
    yield 2;
}

// ── Classes ───────────────────────────────────────────────────────

class Animal {
    constructor(name) {
        this.name = name;
    }

    speak() {
        return `${this.name} makes a noise.`;
    }

    static create(name) {
        return new Animal(name);
    }

    get displayName() {
        return this.name.toUpperCase();
    }

    set displayName(value) {
        this.name = value;
    }
}

class ValidationError extends Error {
    constructor(message, field) {
        super(message);
        this.field = field;
    }
}

// ── Arrow functions ───────────────────────────────────────────────

/**
 * Multiply two numbers.
 * @param {number} a - First number.
 * @param {number} b - Second number.
 * @returns {number} The product.
 */
const multiply = (a, b) => a * b;

const asyncTransform = async (data) => {
    return data.map(x => x * 2);
};

// ── Constants ─────────────────────────────────────────────────────

const MAX_RETRIES = 3;

let mutableCounter = 0;

var legacyFlag = true;

// ── Exports (ESM) ─────────────────────────────────────────────────

export function formatDate(date) {
    return date.toISOString();
}

export class EventBus {
    constructor() {
        this.handlers = {};
    }

    emit(event, data) {
        const fns = this.handlers[event] || [];
        fns.forEach(fn => fn(data));
    }
}

export const VERSION = "1.0.0";

export const processItems = async (items) => {
    return items.filter(Boolean);
};

export default function main() {
    console.log("main");
}
