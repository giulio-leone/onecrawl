/**
 * Origin-based request queue with concurrency control.
 * Limits parallel requests per origin for polite crawling.
 */

interface QueuedRequest<T> {
  execute: () => Promise<T>;
  resolve: (value: T) => void;
  reject: (reason: unknown) => void;
}

interface OriginState {
  pending: Array<QueuedRequest<unknown>>;
  active: number;
}

/**
 * Manages per-origin concurrency limits using a FIFO queue.
 * When a request completes, the next pending request is dequeued.
 */
export class OriginQueue {
  private origins = new Map<string, OriginState>();
  private maxConcurrency: number;

  constructor(maxConcurrency = 6) {
    this.maxConcurrency = maxConcurrency;
  }

  /** Enqueue a request for the given origin. */
  enqueue<T>(origin: string, execute: () => Promise<T>): Promise<T> {
    const state = this.getOrCreate(origin);

    if (state.active < this.maxConcurrency) {
      return this.run(origin, state, execute);
    }

    return new Promise<T>((resolve, reject) => {
      state.pending.push({
        execute: execute as () => Promise<unknown>,
        resolve: resolve as (v: unknown) => void,
        reject,
      });
    });
  }

  private getOrCreate(origin: string): OriginState {
    let state = this.origins.get(origin);
    if (!state) {
      state = { pending: [], active: 0 };
      this.origins.set(origin, state);
    }
    return state;
  }

  private async run<T>(
    origin: string,
    state: OriginState,
    execute: () => Promise<T>,
  ): Promise<T> {
    state.active++;
    try {
      return await execute();
    } finally {
      state.active--;
      this.dequeue(origin, state);
    }
  }

  private dequeue(origin: string, state: OriginState): void {
    if (state.pending.length === 0) {
      if (state.active === 0) this.origins.delete(origin);
      return;
    }

    const next = state.pending.shift()!;
    state.active++;

    next
      .execute()
      .then(next.resolve, next.reject)
      .then(
        () => {
          state.active--;
          queueMicrotask(() => this.dequeue(origin, state));
        },
        (err: unknown) => {
          state.active--;
          queueMicrotask(() => this.dequeue(origin, state));
          throw err;
        },
      )
      .catch(() => {
        /* error boundary: dequeue errors must not propagate */
      });
  }
}
