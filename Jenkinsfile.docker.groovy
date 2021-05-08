pipeline {
    agent any
    options {
        ansiColor('xterm')
    }
    environment {
        TOKEN=credentials("docker-hub-access-token")
    }
    parameters {
        string(name: 'LINUX_RELEASE_TAR_URL', description: 'Where do I download the Schnooty agent from?')
        string(name: 'VERSION', description: 'What will the Docker tag be?')
    }
    stages {
      stage("Build Docker image") {
        steps {
          sh "docker build --no-cache --build-arg LINUX_RELEASE_TAR_URL=${params.LINUX_RELEASE_TAR_URL} . -t schnooty/schnooty"
        }
      }
      
      stage("Tag the release image") {
        steps {
          sh "docker tag schnooty/schnooty:latest schnooty/schnooty:${params.VERSION}"
        }
      }

      stage("Push the image to Docker Hub") {
        steps {
          sh "echo \$TOKEN | docker login --username schnooty --password-stdin"
          sh "docker push schnooty/schnooty:${params.VERSION}"
        }
      }
    }
}
