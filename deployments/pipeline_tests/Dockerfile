FROM sapmachine:17.0.13-jre-ubuntu-jammy as tool_setup 

USER root

ENV DEBIAN_FRONTEND=noninteractive
ENV TZ=Etc/UTC

# Install core utilities
RUN apt-get update --quiet && \
    apt-get install --quiet --yes --no-install-recommends \
    gnupg \
    ca-certificates \
    apt-transport-https \
    wget \
    curl \
    sudo \
    git \
    unzip \
    graphviz \
    tree \
    software-properties-common \
    libarchive-dev && \

    rm -rf /var/lib/apt/lists/*

# Install Apptainer (formerly Singularity)
#RUN add-apt-repository -y ppa:apptainer/ppa && \ 
#     apt-get update --quiet && \
#     apt-get install --quiet --yes apptainer && \
#     rm -rf /var/lib/apt/lists/*

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
    python \

    salmon \
    boost \
    star \
    macs3 \
    multiqc \
    subread \
    kallisto \
    hisat2 \
    bwa \
    bowtie2 \
    fastqc \
    gawk \
    samtools \
    mamba \
    nf-test \
    stringtie \
    black \
    prettier \
    pre-commit \
    pytest-workflow \
    snakemake \
    airflow  \
    trimmomatic \
    picard \
    gatk4 \
    snpeff \
    cnvkit && \

    conda clean --all --force-pkgs-dirs --yes


# Install R and required dependencies
RUN apt-get update --quiet && \
    apt-get install --quiet --yes --no-install-recommends \
    tzdata \
    r-base \
    libxml2-dev \
    libcurl4-openssl-dev \
    libssl-dev \
    libfontconfig1-dev \
    libharfbuzz-dev \
    libfribidi-dev \
    libfreetype6-dev \
    libpng-dev \
    libtiff5-dev \
    libjpeg-dev \
    libgit2-dev \
    libglpk-dev \
    make \
    build-essential && \
    rm -rf /var/lib/apt/lists/*


# Create a writable library path for R packages
ENV R_LIBS_USER=/usr/local/lib/R/site-library

# Ensure the library path is writable
RUN mkdir -p $R_LIBS_USER && \
    chmod -R 777 $R_LIBS_USER

# Install R packages
RUN R -e "install.packages(c('BiocManager', 'ggplot2'), repos='http://cran.rstudio.com/')" && \
    R -e "BiocManager::install(c('DESeq2', 'tximport', 'apeglm', 'edgeR', 'limma', 'EnhancedVolcano'))"



# Pin Nextflow version if necessary
ENV NXF_EDGE=1
ENV NXF_VER=24.02.0-edge
RUN nextflow self-update && nextflow -version


# Set working directory
WORKDIR /workspace

# Clone  repository using the token and setup scripts 
RUN git clone https://github.com/TracerBio/tracer-workflow-templates.git /tmp/temp-scripts \
    && mkdir -p /workspace/bashrc_scripts && cp -R /tmp/temp-scripts/shell-tracer-autoinstrumentation/ /workspace/bashrc_scripts \
    && mkdir -p /workspace/nextflow_scripts && cp -R /tmp/temp-scripts/nextflow-tracer-autoinstrumentation/ /workspace/nextflow_scripts \
    && mkdir -p /workspace/tracer-workflow-templates/data && cp -R /tmp/temp-scripts/data/ /workspace/tracer-workflow-templates \
    && mkdir -p /workspace/data && cp -R /tmp/temp-scripts/data/ /workspace/data \
    && rm -rf /tmp/temp-scripts \

    && chmod -R +x /workspace/bashrc_scripts \
    && chmod -R +x /workspace/nextflow_scripts


# Set environment variables for Nextflow work directory
ENV NXF_WORK=/workspace/nextflow_work

RUN unset JAVA_TOOL_OPTIONS

RUN export PS1='\t -> '

# Default to Bash shell
ENTRYPOINT ["tail", "-f", "/dev/null"]
