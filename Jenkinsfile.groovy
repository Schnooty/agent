def AGENT_DIR = "./"
def GITHUB_CLIENT_URL = "https://github.com/cli/cli/releases/download/v1.9.2/gh_1.9.2_linux_amd64.tar.gz"
def GITHUB_CLIENT_TAR = "gh_1.9.2_linux_amd64.tar.gz"
def GITHUB_CLIENT_PATH = "gh_1.9.2_linux_amd64/bin/gh"
def GITHUB_CLIENT = "./gh_1.9.2_linux_amd64/bin/gh"
def RUST_IMAGE = 'rust:1.51';

pipeline {
  agent none
  environment {
    TOKEN=credentials("github-release-access-token")
    HOME="."
  }
  stages {
    stage('Run unit tests') {
      agent { docker { image RUST_IMAGE } }
      steps {
        dir(AGENT_DIR) {
          sh 'cargo test'
        }
      }
    }

    stage('Build release target') {
      agent { docker { image RUST_IMAGE } }
      steps {
        dir(AGENT_DIR) {
          sh 'cargo build --release --target x86_64-unknown-linux-gnu'
        }
      }
    }

    stage('Produce releasable artefacts') {
      agent { docker { image RUST_IMAGE } }
      steps {
        script {
          dir(AGENT_DIR) {
            sh 'cargo install cargo-get'
            sh 'rm -f *.tar'

            if (isReleaseBuild()) {
              def version = sh(script: 'cargo get version', returnStdout: true).trim()
              def sha = env.GIT_COMMIT.substring(0, 8)
              def releaseFilename = "schnooty-agent-linux-x64-64-${version}-${sha}.tar";
              sh "cp target/x86_64-unknown-linux-gnu/release/schnooty schnooty"
              sh "tar cf ${releaseFilename} schnooty README.md config.toml"
            } else {
              println 'Not master so no need to produce artefacts'
            }
          }
        }
      }
    }

    stage('Upload the release') {
      agent { docker { image RUST_IMAGE } }
      steps {
        script {
          if (isReleaseBuild()) {
            dir(AGENT_DIR) {
              sh 'cargo install cargo-get'

              println "Installing the GitHub client"

              sh "wget ${GITHUB_CLIENT_URL}"
              sh "tar xzf ${GITHUB_CLIENT_TAR}"
              sh "chmod +x $GITHUB_CLIENT_PATH"

              sh "echo \$TOKEN | ${GITHUB_CLIENT} auth login --with-token"

              def cargoVersion = sh(script: 'cargo get version', returnStdout: true).trim()
              def sha = env.GIT_COMMIT.substring(0, 8)
              def gitVersion = "${cargoVersion}-${sha}"

              sh "${GITHUB_CLIENT} release create -p ${gitVersion} ./*.tar"
            }
          }
        }
      }
    }
  }
}

def isReleaseBuild() {
  return env.GIT_BRANCH == 'master';
}
