use r#async::{Runtime, spawn};
use std::sync::{Arc, Mutex};

#[test]
fn test_global_spawn_basic() {
    let rt = Runtime::new();
    let completed = Arc::new(Mutex::new(false));
    let completed_clone = completed.clone();

    rt.block_on(async move {
        spawn(async move {
            *completed_clone.lock().unwrap() = true;
        });
    });

    assert!(
        *completed.lock().unwrap(),
        "Spawned task should have completed"
    );
}

#[test]
fn test_global_spawn_multiple() {
    let rt = Runtime::new();
    let counter = Arc::new(Mutex::new(0));

    let c1 = counter.clone();
    let c2 = counter.clone();
    let c3 = counter.clone();

    rt.block_on(async move {
        spawn(async move {
            *c1.lock().unwrap() += 1;
        });

        spawn(async move {
            *c2.lock().unwrap() += 10;
        });

        spawn(async move {
            *c3.lock().unwrap() += 100;
        });
    });

    assert_eq!(
        *counter.lock().unwrap(),
        111,
        "All spawned tasks should execute"
    );
}

#[test]
fn test_global_spawn_nested() {
    let rt = Runtime::new();
    let values = Arc::new(Mutex::new(Vec::new()));

    let v0 = values.clone();
    let v1 = values.clone();
    let v2 = values.clone();
    let v3 = values.clone();

    rt.block_on(async move {
        v0.lock().unwrap().push(1);

        spawn(async move {
            v1.lock().unwrap().push(2);

            spawn(async move {
                v2.lock().unwrap().push(3);
            });
        });

        spawn(async move {
            v3.lock().unwrap().push(4);
        });
    });

    let mut vals = values.lock().unwrap().clone();
    vals.sort();
    assert_eq!(vals, vec![1, 2, 3, 4], "All nested spawns should execute");
}

#[test]
fn test_global_spawn_from_spawned_task() {
    let rt = Runtime::new();
    let counter = Arc::new(Mutex::new(0));

    let c1 = counter.clone();
    let c2 = counter.clone();

    rt.block_on(async move {
        // Spawn a task that itself spawns another task
        spawn(async move {
            *c1.lock().unwrap() += 1;

            // Spawn from within a spawned task
            spawn(async move {
                *c2.lock().unwrap() += 10;
            });
        });
    });

    assert_eq!(*counter.lock().unwrap(), 11, "Nested spawn should work");
}

#[test]
#[should_panic(expected = "spawn() called outside of a runtime context")]
fn test_global_spawn_panics_outside_runtime() {
    // Calling spawn outside of a runtime context should panic
    spawn(async {
        println!("This should never run");
    });
}

#[test]
fn test_global_spawn_with_return_values() {
    let rt = Runtime::new();
    let results = Arc::new(Mutex::new(Vec::new()));

    let r1 = results.clone();
    let r2 = results.clone();

    rt.block_on(async move {
        spawn(async move {
            let value = compute_value(5);
            r1.lock().unwrap().push(value);
        });

        spawn(async move {
            let value = compute_value(10);
            r2.lock().unwrap().push(value);
        });
    });

    let mut res = results.lock().unwrap().clone();
    res.sort();
    assert_eq!(res, vec![25, 100], "Computed values should be correct");
}

fn compute_value(x: i32) -> i32 {
    x * x
}

#[test]
fn test_spawn_from_separate_async_function() {
    let rt = Runtime::new();
    let counter = Arc::new(Mutex::new(0));

    let c = counter.clone();
    rt.block_on(async move {
        // Appeler une autre fonction async qui utilise spawn
        do_work_with_spawn(c).await;
    });

    assert_eq!(
        *counter.lock().unwrap(),
        42,
        "Spawn from separate function should work"
    );
}

async fn do_work_with_spawn(counter: Arc<Mutex<i32>>) {
    let c1 = counter.clone();
    let c2 = counter.clone();

    // spawn devrait fonctionner ici car on est dans le contexte de block_on
    spawn(async move {
        *c1.lock().unwrap() += 10;
    });

    spawn(async move {
        *c2.lock().unwrap() += 32;
    });
}

#[test]
fn test_spawn_from_nested_function_calls() {
    let rt = Runtime::new();
    let values = Arc::new(Mutex::new(Vec::new()));

    let v = values.clone();
    rt.block_on(async move {
        v.lock().unwrap().push(1);
        nested_function_a(v).await;
    });

    let mut vals = values.lock().unwrap().clone();
    vals.sort();
    assert_eq!(
        vals,
        vec![1, 2, 3, 4],
        "Spawn from nested function calls should work"
    );
}

async fn nested_function_a(values: Arc<Mutex<Vec<i32>>>) {
    let v1 = values.clone();
    let v2 = values.clone();

    spawn(async move {
        v1.lock().unwrap().push(2);
    });

    nested_function_b(v2).await;
}

async fn nested_function_b(values: Arc<Mutex<Vec<i32>>>) {
    let v1 = values.clone();
    let v2 = values.clone();

    spawn(async move {
        v1.lock().unwrap().push(3);
    });

    spawn(async move {
        v2.lock().unwrap().push(4);
    });
}
