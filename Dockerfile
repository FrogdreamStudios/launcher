FROM ubuntu:22.04

RUN apt-get update && apt-get install -y \
    curl \
    build-essential \
    libgtk-3-dev \
    libglib2.0-dev \
    libsoup-3.0-dev \
    libssl-dev \
    libjavascriptcoregtk-4.1-dev \
    libwebkit2gtk-4.1-dev \
    libxdo-dev \
    pkg-config \
    zip \
    xdotool \
    git \
    ca-certificates \
 && rm -rf /var/lib/apt/lists/*

RUN curl https://sh.rustup.rs -sSf | bash -s -- -y --default-toolchain nightly
ENV PATH="/root/.cargo/bin:${PATH}"
RUN rustup component add rustfmt clippy

WORKDIR /app
COPY . .

RUN cargo build --release --locked --features "desktop"

RUN mkdir dist && \
    cp target/release/DreamLauncher dist/ && \
    zip -j DreamLauncher-linux.zip dist/DreamLauncher

CMD ["bash", "-c", "ls -lh DreamLauncher-linux.zip && echo Build completed"]