package analytics;

import domain.Box;
import domain.Event;
import std.math as math;

function wrap_float(value: Float): Box<Float> {
    return Box<Float>(value);
}

function transform_pair<T, U>(left: T, right: T, f: (T, T) -> U): U {
    return f(left, right);
}

function sum_history(seed: Integer): Integer {
    mut history: List<Integer> = List<Integer>();
    mut current: Integer = seed;
    mut i: Integer = 0;
    while (i < 4) {
        history.push(current);
        current = current + i + 1;
        i = i + 1;
    }

    mut total: Integer = 0;
    for (value in history) {
        total = total + value;
    }
    return total;
}

module Api {
    module V2 {
        function weighted_ratio(total: Integer, weight: Float): Float {
            return to_float(total) / weight;
        }

        function build_event(label: String, total: Integer): Event {
            return Event.Metric(label, weighted_ratio(total, 2.0));
        }
    }
}

function build_snapshot(flag: Boolean, seed: Integer): Result<Option<Box<Float>>, String> {
    if (!flag) {
        return Result.error("disabled");
    }
    if (seed % 2 == 0) {
        return Result.ok(Option.some(wrap_float(Api.V2.weighted_ratio(seed, 2.0))));
    }
    return Result.ok(Option.none());
}
