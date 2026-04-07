package analytics;

import std.math as math;

module AsyncApi {
    async function project_score(value: Integer): Task<Float> {
        base: Float = to_float(value + sum_history(value)) / 3.0;
        return math.max(base, value);
    }
}
