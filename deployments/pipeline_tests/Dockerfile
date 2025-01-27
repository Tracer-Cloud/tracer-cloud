# Stage 1: Nextflow Setup
FROM debian:bullseye-slim AS tool_setup

USER root

# Install core utilities
RUN apt-get update --quiet && \
    apt-get install --quiet --yes --no-install-recommends \
        ca-certificates \
        apt-transport-https \
        wget \
        curl \
        sudo \
        git \
        unzip \
        graphviz \
        tree && \
    rm -rf /var/lib/apt/lists/*

# Install Apptainer (formerly Singularity)
RUN apt-get update --quiet && \
    apt-get install --quiet --yes --no-install-recommends \
        software-properties-common && \
    add-apt-repository -y ppa:apptainer/ppa && \
    apt-get install --quiet --yes apptainer && \
    rm -rf /var/lib/apt/lists/*

# Install Miniconda
RUN wget --quiet https://repo.anaconda.com/miniconda/Miniconda3-latest-Linux-x86_64.sh && \
    bash Miniconda3-latest-Linux-x86_64.sh -b -p /opt/conda && \
    rm Miniconda3-latest-Linux-x86_64.sh
ENV PATH="/opt/conda/bin:$PATH"

# Configure conda and install Nextflow + required packages
RUN conda config --add channels defaults && \
    conda config --add channels bioconda && \
    conda config --add channels conda-forge && \
    conda config --set channel_priority strict && \
    conda install --quiet --yes --name base \
        nextflow \
        nf-core \
        python && \
    conda clean --all --force-pkgs-dirs --yes


# Install additional tools and libraries
RUN apt-get update --quiet && \
    apt-get install --quiet --yes --no-install-recommends \
        r-base \
        libxml2-dev \
        libcurl4-openssl-dev \
        libssl-dev \
        libfontconfig1-dev && \
    R -e "install.packages(c('BiocManager', 'ggplot2'), repos='http://cran.rstudio.com/')" && \
    R -e "BiocManager::install(c('DESeq2', 'EnhancedVolcano'))" && \
    rm -rf /var/lib/apt/lists/*



# Pin Nextflow version if necessary
ENV NXF_EDGE=1
ENV NXF_VER=24.02.0-edge
RUN nextflow self-update && nextflow -version

# Stage 2: Custom Tools and Bashrc
FROM tool_setup AS script_setups

# Set working directory
WORKDIR /workspace


# Pass the GitHub token as a build argument
ARG GITHUB_TOKEN

# Clone  repository using the token and setup scripts 
RUN git clone https://${GITHUB_TOKEN}@github.com/TracerBio/tracer-workflow-templates.git /tmp/temp-scripts \
    && mkdir /workspace/bashrc_scripts && cp -R /tmp/temp-scripts/shell-tracer-autoinstrumentation/ /workspace/bashrc_scripts \
    && mkdir /workspace/bashrc_scripts && cp -R /tmp/temp-scripts/nextflow-tracer-autoinstrumentation/ /workspace/nextflow_scripts \
    && rm -rf /tmp/temp-scripts \

    && chmod -R +x /workspace/bashrc_scripts \
    && chmod -R +x /workspace/nextflow_scripts


# Set environment variables for Nextflow work directory
ENV NXF_WORK=/workspace/nextflow_work

# Default to Bash shell
ENTRYPOINT ["/bin/bash"]
