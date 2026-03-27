package runtime;

import analytics as analytics;
import domain.Box;
import std.fs.*;
import std.string.*;
import std.time.*;

function runtime_cat(a: String, b: String): String {
    return Str.concat(a, b);
}

async function render_report(label: String, value: Integer): Task<String> {
    score: Float = await analytics.AsyncApi.project_score(value);
    detail: Result<Option<Box<Float>>, String> = analytics.build_snapshot(true, value);
    nested: String = match (detail) {
        Result.Ok(value) => match (value) {
            Some(boxed) => "nested:" + to_string(boxed.get()),
            None => "nested:none",
        },
        Result.Error(err) => "nested:error:" + err,
    };
    return runtime_cat(
        runtime_cat(label, "|score="),
        runtime_cat(to_string(score), runtime_cat("|", nested))
    );
}

function persist_report(filename: String, body: String): String {
    ok: Boolean = File.write(filename, body);
    if (!ok) {
        return "write-failed";
    }
    copy: String = File.read(filename);
    deleted: Boolean = File.delete(filename);
    if (!deleted) {
        return "delete-failed";
    }
    return copy;
}

function stamp(): String {
    return "unix=" + to_string(Time.unix());
}
