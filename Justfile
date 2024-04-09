export WORKER_ID := "pueued-linux_china"

# Bump all deps, including incompatible version upgrades
bump:
    just ensure_installed upgrade
    cargo update
    cargo upgrade --incompatible
    cargo test --workspace

# Run the test suite with nexttest
nextest:
    just ensure_installed nextest
    cargo nextest run --workspace

ensure_installed *args:
    #!/bin/bash
    cargo --list | grep -q {{ args }}
    if [[ $? -ne 0 ]]; then
        echo "error: cargo-{{ args }} is not installed"
        exit 1
    fi

lint:
    cargo fmt
    cargo clippy --all --tests

start-server:
   cargo run --bin pueued

commit-register:
   nats pub 'pueued.worker.pueued-linux_china' pong

exeucte-java-version:
   nats pub 'pueued.worker.pueued-linux_china' '{"command": "java --version", "path": "/tmp","envs": {}, "start_immediately": false, "stashed": false, "group": "default", "enqueue_at": null, "dependencies": [], "label": "task-xxxxx","print_task_id": false}'