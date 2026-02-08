/**
 * Calculate the sum of numbers.
 * @param {number[]} numbers - Array of numbers.
 * @returns {number} The sum.
 */
function sum(numbers) {
    return numbers.reduce((a, b) => a + b, 0);
}

class EventBus {
    constructor() {
        this.handlers = {};
    }

    emit(event, data) {
        const fns = this.handlers[event] || [];
        fns.forEach(fn => fn(data));
    }
}

export default EventBus;
