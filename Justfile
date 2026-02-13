version := `cat Cargo.toml | grep version | head -1 | cut -d " " -f 3 | tr -d "\""`
chartVersion := `cat helm-chart/Chart.yaml | yq -r '.version'`
image := "tinyops/cronbird"
trivyReportFile := "docs/security/trivy-scan-report.txt"

init:
    rustup component add clippy
    cargo install cargo-llvm-cov

build-dev-image:
    docker build --progress=plain --platform=linux/amd64 .

format:
    cargo fmt

lint: format
    cargo clippy -- -D warnings

test:
    cargo test

build: lint && test
    cargo build

########################################
# DEV ENV
########################################

cleanup:
    rm -f cronbird-*.tgz

run:
    cargo run

start-dev-image:
    docker compose -f docker-compose-dev.yml up -d --build --force-recreate

stop-dev-image:
    docker compose -f docker-compose-dev.yml down

########################################
# HELM CHART
########################################

test-chart:
    @echo "Testing dynamic mode..."
    @helm template helm-chart/ --set config.allowDynamic=true > /dev/null
    @echo "Testing static mode with identities..."
    @helm template helm-chart/ --set 'config.identities=job1\,job2' > /dev/null
    @echo "✓ Chart validation passed"

build-chart: test-chart
    helm package helm-chart/ --app-version {{ version }}

release-chart: build-chart
    rm -rf helm-repo
    git clone git@github.com:tinyops-ru/tinyops-ru.github.io.git helm-repo
    bash -euo pipefail -c '\
        cd helm-repo && \
        cp ../cronbird-{{ chartVersion }}.tgz helm-charts/ && \
        helm repo index helm-charts/ && \
        if [ -z "$(git status --porcelain)" ]; then \
            echo "Chart cronbird-{{ chartVersion }} already published, skipping." && \
            exit 0; \
        fi && \
        git add helm-charts/ && \
        git commit -m "Add helm chart: cronbird-{{ chartVersion }}" && \
        git push'
    rm -rf helm-repo

########################################
# SECURITY
########################################

trivy:
    trivy image --severity HIGH,CRITICAL {{ image }}:{{ version }}

########################################
# RELEASE

# #######################################
build-release-image: lint && test
    docker build --progress=plain --platform=linux/amd64 -t {{ image }}:{{ version }} .

trivy-save-reports:
    trivy -v > {{ trivyReportFile }}
    trivy config Dockerfile >> {{ trivyReportFile }}
    trivy image --severity HIGH,CRITICAL {{ image }}:{{ version }} >> {{ trivyReportFile }}

release: build-release-image && release-chart
    docker push {{ image }}:{{ version }}
    just trivy-save-reports
