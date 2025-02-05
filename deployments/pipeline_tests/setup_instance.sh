#!/bin/bash

set -e  # Exit immediately if a command exits with a non-zero status.
set -u  # Treat unset variables as an error.

# Set non-interactive mode for apt-get
export DEBIAN_FRONTEND=noninteractive
export TZ=Etc/UTC

# Update and install core utilities
echo "Updating and installing core utilities..."
sudo apt-get update --quiet
sudo apt-get install --quiet --yes --no-install-recommends \
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
    libarchive-dev

# Install Java (SapMachine) Use this instead. https://rushiinfotech.in/java-17-installation-on-ubuntu-22-04-lts/ ??
echo "Installing Java ..."
sudo apt install --quiet --yes openjdk-17-jdk

# Install Miniconda
# echo "Installing Miniconda..."
# wget --quiet https://repo.anaconda.com/miniconda/Miniconda3-latest-Linux-x86_64.sh
# sudo bash Miniconda3-latest-Linux-x86_64.sh -b -p /opt/conda
# rm Miniconda3-latest-Linux-x86_64.sh
# # export PATH="/opt/conda/bin:$PATH"
# echo 'export PATH="/opt/conda/bin:$PATH"' >> ~/.bashrc
# 
# sudo chown -R $USER:$USER /opt/conda/
# 
# source ~/.bashrc

# Check if Conda is installed
if ! command -v conda &> /dev/null
then
    # Install Miniconda
    echo "Installing Miniconda..."
    wget --quiet https://repo.anaconda.com/miniconda/Miniconda3-latest-Linux-x86_64.sh
    sudo bash Miniconda3-latest-Linux-x86_64.sh -b -p /opt/conda
    rm Miniconda3-latest-Linux-x86_64.sh
    
    # Add Conda to PATH
    echo 'export PATH="/opt/conda/bin:$PATH"' >> ~/.bashrc
    sudo chown -R $USER:$USER /opt/conda/
    
    # Apply the changes to the current shell session
    source ~/.bashrc
else
    echo "Conda is already installed."
fi


# Configure Conda and install Nextflow + packages
echo "Configuring Conda and installing packages..."
conda config --add channels defaults
conda config --add channels bioconda
conda config --add channels conda-forge
conda config --set channel_priority strict

# optional
conda install -n base libarchive -c main --force-reinstall --solver classic

echo "Installing First sets..."
conda install --quiet --yes --name base \
    nextflow \
    nf-core \
    python \
    salmon \
    deeptools \



echo "Installing Second sets..."
conda install --quiet --yes --name base \
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



echo "Installing Third sets..."
conda install --quiet --yes --name base \
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


echo "Installing Last sets..."
conda install --quiet --yes --name base \
    airflow \
    trimmomatic \
    picard \
    gatk4 \
    snpeff \
    cnvkit


echo "Cleaning up..."
conda clean --all --force-pkgs-dirs --yes

# Install R and dependencies
echo "Installing R and dependencies..."
sudo apt-get update --quiet
sudo apt-get install --quiet --yes --no-install-recommends \
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
    build-essential
sudo rm -rf /var/lib/apt/lists/*

# Configure writable R library path
# export R_LIBS_USER=/usr/local/lib/R/site-library
echo 'export R_LIBS_USER=/usr/local/lib/R/site-library' >> ~/.bashrc
source ~/.bashrc

sudo mkdir -p $R_LIBS_USER
sudo chmod -R 777 $R_LIBS_USER

# Install R packages Would need to use 4.4 and above because 
# ‘MASS’ version 7.3-64 is in the repositories but depends on R (>= 4.4.0) so this doesn't quite work
echo "Installing R packages..."
R -e "install.packages(c('BiocManager', 'ggplot2'), repos='http://cran.rstudio.com/')"
R -e "BiocManager::install(c('DESeq2', 'tximport', 'apeglm', 'edgeR', 'limma', 'EnhancedVolcano'))"


# Pin Nextflow version and verify
# export NXF_EDGE=1
# export NXF_VER=24.02.0-edge
echo 'export NXF_EDGE=1' >> ~/.bashrc
echo 'export NXF_VER=24.02.0-edge' >> ~/.bashrc
source ~/.bashrc


nextflow self-update
nextflow -version


# Set Nextflow work directory
# docker version
# export NXF_WORK=/workspace/nextflow_work

export NXF_WORK=/home/ubuntu/nextflow_work
# Clean up
unset JAVA_TOOL_OPTIONS
echo "Setup complete."



# Some notes:
# For Bashrc make sure you set the dir properly. 
# You should be able provide a valid path to data/ dir where the script can cd into
