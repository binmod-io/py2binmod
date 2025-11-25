ARG RUST_VERSION=1.91
ARG PYTHON_VERSION=3.12
FROM registry.gitlab.com/wizrds/rust-python-docker:${RUST_VERSION}_${PYTHON_VERSION} AS builder

RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
    && apt-get install -y --no-install-recommends locales \
    && echo "en_US.UTF-8 UTF-8" > /etc/locale.gen \
    && locale-gen \
    && rm -rf /var/lib/apt/lists/*

ENV LANG=en_US.UTF-8 \
    LANGUAGE=en_US:en \
    LC_ALL=en_US.UTF-8

RUN python -m venv /opt/venv
ENV VIRTUAL_ENV="/opt/venv"
ENV PATH="$VIRTUAL_ENV/bin:$PATH"

RUN /opt/venv/bin/pip install --upgrade pip setuptools wheel uv maturin

WORKDIR /build
COPY . .

RUN maturin build --release -o /build/dist

FROM registry.gitlab.com/wizrds/rust-python-docker:${RUST_VERSION}_${PYTHON_VERSION} AS runtime

RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
    # Remove imagemagick due to https://security-tracker.debian.org/tracker/CVE-2019-10131
    && apt-get purge -y imagemagick imagemagick-6-common \
    && apt-get install -y jq bat curl git locales \
    && echo "en_US.UTF-8 UTF-8" > /etc/locale.gen \
    && locale-gen \
    && update-locale LANG=en_US.UTF-8 LC_ALL=en_US.UTF-8 \
    && rm -rf /var/lib/apt/lists/*

ENV LANG=en_US.UTF-8 \
    LANGUAGE=en_US:en \
    LC_ALL=en_US.UTF-8

RUN useradd -u 1000 -G sudo -U -m -s /bin/bash py2binmod \
    && echo "py2binmod ALL=(ALL) NOPASSWD: /bin/chown" >> /etc/sudoers

COPY --from=builder --chown=py2binmod:py2binmod /opt/venv /opt/venv

ENV VIRTUAL_ENV="/opt/venv"
ENV PATH="$VIRTUAL_ENV/bin:$PATH"
RUN echo 'export PATH=/opt/venv/bin:$PATH' >> /home/py2binmod/.profile
RUN /opt/venv/bin/pip install --upgrade pip setuptools wheel uv

COPY --from=builder /build/dist /tmp/dist
RUN uv pip install /tmp/dist/*.whl

RUN mkdir /app && chown py2binmod:py2binmod /app
WORKDIR /app

USER py2binmod

RUN rustup target add wasm32-unknown-unknown
RUN rustup target add wasm32-wasip1

ENTRYPOINT ["py2binmod"]
CMD ["--help"]