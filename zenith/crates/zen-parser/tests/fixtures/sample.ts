export interface Handler<T> {
    handle(input: T): Promise<void>;
    readonly name: string;
}

export type Result<T> = { ok: true; value: T } | { ok: false; error: Error };

export async function processItems<T>(
    items: T[],
    handler: Handler<T>,
): Promise<Result<T[]>> {
    return { ok: true, value: items };
}

export default class EventEmitter {
    private listeners: Map<string, Function[]> = new Map();

    on(event: string, callback: Function): void {
        // register listener
    }

    emit(event: string, data: unknown): void {
        // emit event
    }
}
