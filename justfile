# Workstation build, Compose, and VM recipes. VM tools live in justfile.vm.
set shell := ["bash", "-euo", "pipefail", "-c"]
set positional-arguments

vm := env_var_or_default("RATINGS_VM", "ratings-lab")
cpus := env_var_or_default("RATINGS_VM_CPUS", "4")
memory := env_var_or_default("RATINGS_VM_MEMORY", "12G")
disk := env_var_or_default("RATINGS_VM_DISK", "60G")
repo := justfile_directory()
operator_repo := absolute_path(env_var_or_default("RATINGS_OPERATOR_REPO", parent_directory(repo) / "container-runner-operator"))
vm_repo := "/home/ubuntu/app-center-ratings"
vm_operator_repo := "/home/ubuntu/container-runner-operator"
kubeconfig_path := home_directory() / ".kube/ratings-local.yaml"
kubeconfig_timeout := env_var_or_default("RATINGS_KUBECONFIG_TIMEOUT", "30s")
vm_path := "/home/ubuntu/.cargo/bin:/home/ubuntu/.local/bin:/snap/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"

# List workstation recipes; see doc/charming.md for first-time installation.
default:
    @just --list

# Build the release binary on the workstation, not inside Multipass.
build:
    cargo build --release

docker-build:
    docker build -f docker/prod/Dockerfile .

# Start the Compose stack; `up` below still starts the Multipass VM.
compose-up:
    docker compose up

up-detached *args:
    docker compose up --detach "$@"

down:
    docker compose down

# Unit tests only; no database or Compose stack is started.
test:
    cargo test --lib

db-test:
    APP_HOST='0.0.0.0' \
        APP_PORT='8080' \
        APP_JWT_SECRET='deadbeef' \
        APP_SNAPCRAFT_IO_URI='localhost:11111/' \
        APP_POSTGRES_URI='postgresql://migration_user:strongpassword@localhost:5432/ratings' \
        cargo test --lib --features db_tests

integration-test *args: clear-db-data
    APP_JWT_SECRET='deadbeef' \
        MOCK_ADMIN_URL='http://127.0.0.1:11111/__admin__/register-snap' \
        HOST='0.0.0.0' \
        PORT='8080' \
        cargo test --test '*' "$@"

test-all *args: db-test
    just --justfile "{{ repo }}/justfile" integration-test "$@"

# Wait for the stack's healthchecks, failing on unhealthy/exited containers or timeout.
wait-for-server timeout="600":
    docker compose up --no-build --wait --wait-timeout "$1"

# Use the non-watching CI image; always tear down, including on startup/test failure.
ci-test:
    #!/usr/bin/env bash
    set -euo pipefail
    export COMPOSE_FILE="{{ repo }}/docker-compose.yml:{{ repo }}/docker-compose.ci.yml"
    cleanup() {
        local status=$? cleanup_status=0
        trap - EXIT
        if (( status != 0 )); then
            echo "CI failed; collecting container status and logs before teardown." >&2
            docker compose ps --all || true
            docker compose logs --no-color --tail=200 || true
        fi
        just --justfile "{{ repo }}/justfile" down || cleanup_status=$?
        if (( status == 0 )); then
            status=$cleanup_status
        fi
        exit "$status"
    }
    trap cleanup EXIT
    just --justfile "{{ repo }}/justfile" up-detached --build
    just --justfile "{{ repo }}/justfile" wait-for-server
    just --justfile "{{ repo }}/justfile" test-all

clear-db-data:
    docker compose exec -T db psql -U postgres ratings < tests/clear-db.sql

rebuild-local:
    docker compose build

rm-db:
    docker compose rm db
    docker volume rm -f postgres_data

rm-volumes:
    docker volume rm -f target-cache
    docker volume rm -f cargo-cache

db-shell:
    docker exec -it ratings-db psql -U postgres ratings

ratings-shell:
    docker exec -it ratings bash

#[group("publish")]
charm:
    just vm charm-pack

#[group("publish")]
rock:
    just vm rock

# Install Ubuntu workstation prerequisites (requires just to already be installed).
#[group("dev")]
host-install:
    sudo apt-get update
    sudo apt-get install -y ca-certificates cpu-checker curl git openssh-client openssl python3 snapd
    sudo systemctl enable --now snapd.socket
    if ! snap list just >/dev/null 2>&1; then sudo snap install just --classic --channel=latest/stable; fi
    if ! snap list multipass >/dev/null 2>&1; then sudo snap install multipass --channel=latest/stable; fi
    if ! snap list kubectl >/dev/null 2>&1; then sudo snap install kubectl --classic --channel=1.32/stable; fi
    kvm-ok

# Create or start the VM and mount the application and sidecar operator checkouts.
create:
    #!/usr/bin/env bash
    set -euo pipefail
    if multipass info "{{ vm }}" >/dev/null 2>&1; then
        multipass start "{{ vm }}"
    else
        multipass launch 24.04 --name "{{ vm }}" --cpus "{{ cpus }}" --memory "{{ memory }}" --disk "{{ disk }}"
    fi
    source_path=$(multipass info "{{ vm }}" --format=json | python3 -c 'import json, sys; d=json.load(sys.stdin)["info"][sys.argv[1]]; print(d.get("mounts", {}).get(sys.argv[2], {}).get("source_path", ""))' "{{ vm }}" "{{ vm_repo }}")
    if [ -z "$source_path" ]; then
        multipass mount --type classic "{{ repo }}" "{{ vm }}:{{ vm_repo }}"
    elif [ "$source_path" != "{{ repo }}" ]; then
        printf 'Refusing to replace existing mount from %s at %s\n' "$source_path" "{{ vm_repo }}" >&2
        exit 1
    fi
    just --justfile "{{ repo }}/justfile" operator-mount

# Mount the standalone sidecar checkout into an existing lab; never replace a mount.
operator-mount:
    #!/usr/bin/env bash
    set -euo pipefail
    if ! grep -qx 'name: ratings' "{{ operator_repo }}/charmcraft.yaml"; then
        printf 'Expected the Ratings sidecar checkout at %s. Select feat/ps7 (jj change vpmyx), not the legacy machine main, or set RATINGS_OPERATOR_REPO.\n' "{{ operator_repo }}" >&2
        exit 1
    fi
    source_path=$(multipass info "{{ vm }}" --format=json | python3 -c 'import json, sys; d=json.load(sys.stdin)["info"][sys.argv[1]]; print(d.get("mounts", {}).get(sys.argv[2], {}).get("source_path", ""))' "{{ vm }}" "{{ vm_operator_repo }}")
    if [ -z "$source_path" ]; then
        multipass mount --type classic "{{ operator_repo }}" "{{ vm }}:{{ vm_operator_repo }}"
    elif [ "$source_path" != "{{ operator_repo }}" ]; then
        printf 'Refusing to replace existing operator mount from %s at %s\n' "$source_path" "{{ vm_operator_repo }}" >&2
        exit 1
    fi

# First-time VM setup; deliberately does not depend on just already existing there.
bootstrap:
    multipass exec "{{ vm }}" -- sudo apt-get update
    multipass exec "{{ vm }}" -- sudo apt-get install -y ca-certificates curl git python3 snapd
    multipass exec "{{ vm }}" -- sh -c 'if ! snap list just >/dev/null 2>&1; then sudo snap install just --classic --channel=latest/stable; fi'
    multipass exec --working-directory "{{ vm_repo }}" "{{ vm }}" -- env PATH="{{ vm_path }}" RATINGS_OPERATOR_REPO="{{ vm_operator_repo }}" /snap/bin/just --justfile justfile.vm prepare

# Resume an existing lab; no package installation or redeployment.
up:
    multipass start "{{ vm }}"

stop:
    multipass stop "{{ vm }}"

shell:
    multipass shell "{{ vm }}"

info:
    multipass info "{{ vm }}"

# Run any VM recipe, forwarding arguments without shell re-interpretation.
vm recipe *args:
    multipass exec --working-directory "{{ vm_repo }}" "{{ vm }}" -- env PATH="{{ vm_path }}" RATINGS_OPERATOR_REPO="{{ vm_operator_repo }}" /snap/bin/just --justfile justfile.vm "$@"

# Export admin credentials atomically to a separate host kubeconfig, not ~/.kube/config.
kubeconfig:
    #!/usr/bin/env bash
    set -euo pipefail
    run_bounded() {
        local step="$1" status
        shift
        if timeout --kill-after=5s "{{ kubeconfig_timeout }}" "$@" </dev/null; then
            return 0
        else
            status=$?
        fi
        if [ "$status" -eq 124 ] || [ "$status" -eq 137 ]; then
            printf '%s timed out after %s. Check Multipass/VM connectivity; increase RATINGS_KUBECONFIG_TIMEOUT only if needed.\n' "$step" "{{ kubeconfig_timeout }}" >&2
        else
            printf '%s failed (exit %s). Existing kubeconfig was not replaced.\n' "$step" "$status" >&2
        fi
        return "$status"
    }
    printf 'Inspecting VM %s (timeout %s)...\n' "{{ vm }}" "{{ kubeconfig_timeout }}" >&2
    vm_info=$(run_bounded 'VM inspection' multipass info "{{ vm }}" --format=json)
    vm_ip=$(printf '%s' "$vm_info" | python3 -c 'import ipaddress, json, sys; vm=json.load(sys.stdin)["info"][sys.argv[1]]; vm["state"] == "Running" or sys.exit("VM is not running; run just up first."); ip=next((ip for ip in vm["ipv4"] if not ipaddress.ip_address(ip).is_loopback), None); ip or sys.exit("VM has no usable IPv4 address; check just info."); print(ip)' "{{ vm }}")
    printf 'Exporting kubeconfig from %s (API https://%s:6443, timeout %s)...\n' "{{ vm }}" "$vm_ip" "{{ kubeconfig_timeout }}" >&2
    mkdir -p "$(dirname "{{ kubeconfig_path }}")"
    umask 077
    temporary=$(mktemp "{{ kubeconfig_path }}.XXXXXX")
    trap 'rm -f "$temporary"' EXIT
    # A pipe avoids snap-confine's AppArmor file_inherit denial when just is a snap.
    # pipefail still prevents installing partial output if Multipass fails.
    run_bounded 'Kubeconfig export' multipass exec --no-map-working-directory "{{ vm }}" -- sudo -n k8s config --server "https://$vm_ip:6443" | cat > "$temporary"
    if [ ! -s "$temporary" ]; then
        printf 'VM returned an empty kubeconfig; existing kubeconfig was not replaced.\n' >&2
        exit 1
    fi
    mv "$temporary" "{{ kubeconfig_path }}"
    printf 'Kubeconfig written to %s (API https://%s:6443)\n' "{{ kubeconfig_path }}" "$vm_ip"

# Use only this lab's kubeconfig, regardless of the workstation's current context.
kubectl *args:
    kubectl --kubeconfig "{{ kubeconfig_path }}" "$@"

# Keep the Grafana forward bound to host loopback; leave this terminal open.
grafana:
    kubectl --kubeconfig "{{ kubeconfig_path }}" --namespace cos-lite port-forward --address=127.0.0.1 service/grafana 3000:3000

# Foreground gRPC forwarding for the Juju-managed Ratings application.
ratings port="8080": clean-cache
    kubectl --kubeconfig "{{ kubeconfig_path }}" --namespace ratings-dev port-forward --address=127.0.0.1 service/ratings "$1:8080"

clean-cache:
    just kubectl -n ratings-dev exec ratings-0 \
      -c ratings -- /charm/bin/pebble restart ratings

# Optional API check: requires host uv; keep just ratings running separately.
grpc-smoke target="127.0.0.1:8080":
    uv run "{{ repo }}/dev/grpc_smoke.py" --target "$1"

# Public CA only, atomically exported to ignored dev/build/ingress/edge-ca.pem.
ingress-ca:
    python3 "{{ repo }}/dev/ingress_access.py" ca --vm "{{ vm }}"

# Foreground host-loopback SSH forward through Multipass to the LXD HAProxy port 443.
ingress port="8443":
    python3 "{{ repo }}/dev/ingress_access.py" tunnel --vm "{{ vm }}" --port "$1"

# Keep just ingress running separately; verify CA + hostname + ALPN before gRPC.
ingress-tls-check port="8443":
    python3 "{{ repo }}/dev/ingress_access.py" tls-check --port "$1"

ingress-smoke port="8443":
    just --justfile "{{ repo }}/justfile" ingress-tls-check "$1"
    uv run "{{ repo }}/dev/grpc_smoke.py" --target "ratings.localhost:$1" --ca-file "{{ repo }}/dev/build/ingress/edge-ca.pem"

# Stdlib-only offline orchestration checks; all external commands are mocked.
lab-test:
    python3 -m unittest discover -s "{{ repo }}/dev" -p 'test_*.py' -v

# Stop and snapshot the VM. It remains stopped; mounted source is not snapshotted.
snapshot name="prepared":
    multipass stop "{{ vm }}"
    multipass snapshot "{{ vm }}" --name "$1"

# Restore VM-local state; Multipass prompts before discarding the current state.
restore name:
    multipass stop "{{ vm }}"
    multipass restore "{{ vm }}.$1"
    multipass start "{{ vm }}"

# Permanently delete only this VM and its snapshots, with explicit confirmation.
delete:
    #!/usr/bin/env bash
    set -euo pipefail
    read -r -p 'Permanently delete {{ vm }} and its snapshots? Type the VM name: ' confirmation
    [ "$confirmation" = "{{ vm }}" ] || { printf 'Cancelled.\n'; exit 1; }
    multipass delete --purge "{{ vm }}"
    printf 'The checkout and exported host kubeconfig were not removed. The kubeconfig is now stale.\n'
