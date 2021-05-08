pipeline {
    agent any
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
          sh "Using the image from ${params.LINUX_RELEASE_TAR_URL}"
          sh "docker build --build-arg LINUX_RELEASE_TAR_URL=${params.LINUX_RELEASE_TAR_URL} . -t schnooty"
        }
      }
      
      stage("Tag the release image") {
        steps {
          sh "docker tag schnooty:latest schnooty:${params.VERSION}"
        }
      }

      stage("Push the image to Docker Hub") {
        steps {
          sh "echo $TOKEN | docker login schnooty --password-stdin"
          sh "docker push schnooty:${params.VERSION}"
        }
      }
    }
}
