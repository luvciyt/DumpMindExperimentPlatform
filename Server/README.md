# Server 说明

Server 的组成部分
- docker-compose.yml: docker 部署 postgresql 用于持久化数据
- task-composer: 用于维护任务相关的信息, 包括部署,分发,分发后监控等
- RESTFUL api: 用于常规的与worker的信息
- GRPC: 用于定义双向交互等更先进的信息