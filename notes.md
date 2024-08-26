Pueue和NATS对接：

1. pueued启动时的环境变量问题，尤其是Path，这个是各个命令行搜索要用到，目前考虑使用which进行命令行定位

# pueued的启动流程

* 证书：所有的pueued都使用统一的证书。pueued在启动时如果发现证书不存在，会自动创建对应的证书，可以考虑所有的pueued都使用统一的证书。
* 配置：所有的pueued都使用统一的配置，启动监听端口号

如果制作Docker镜像的话，可以考虑将证书和配置文件放在镜像中。

# pueued对接NATS的启动流程

1. 创建NATS的连接: 从环境变量中读取 `NATS_HOST`的地址
2. 设置本地的worker id: `WORKER_ID`环境变量 或者 随机UUID
3. 订阅worker_id对应的subject: `pueued.worker_id`
4. worker注册：向`pueued.registry`
   subject发送注册消息，消息格式参考[Eureka Register API](https://github.com/Netflix/eureka/wiki/Eureka-REST-operations)
   ，对应的json如下，

```json
{
  "hostName": "xxx",
  "app": "pueued-worker",
  "ipAddr": "192.168.0.2",
  "vipAddress": "10.0.0.3",
  "secureVipAddress": "10.0.1.3",
  "status": "UP",
  "port": 0,
  "securePort": 0,
  "homePageUrl": "http://xxx",
  "statusPageUrl": "http://xxx/info",
  "healthCheckUrl": "http://xxx/health",
  "dataCenterInfo": {
    "name": "MyOwn",
    "metadata": {
      "region": "xxx"
    }
  },
  "metadata": {
    "uuid": "worker-uuid",
    "inbox": "pueued-worker-uuid"
  }
}
```

* pueued关闭：则会向`pueued.registry`发现status为`DOWN`的消息。
* 屏蔽某一pueued: 将其状态设置为`DOWN`，然后不要给其在发送消息

启动流程如下：

1. 启动pueued服务`RUST_LOG=info WORKER_ID=WORKER1 ./target/debug/pueued`，同时会监听`pueued.worker_id` subject。
2. pueued会向`pueued.registry`发送消息，你可以通过`nats subscribe 'pueued.registry'`进行查看
3. 当监听服务接收到`pueued.registry`中的上线消息，请向`pueued.worker_id`发送`pong`消息进行确认。

# 向pueued下发任务

### 简单命令行

通过 `nats publish "pueued.worker.WORKER1" "java --version"` 下发简单命令行，就可以进行测试。 然后使用`pueue status`进行查看。

### JSON

通过JSON方式发送执行命令，格式如下：

```json

{
  "command": "java --version",
  "path": "/tmp",
  "envs": {},
  "start_immediately": false,
  "stashed": false,
  "group": "default",
  "enqueue_at": null,
  "dependencies": [],
  "label": "task-xxxxx",
  "print_task_id": false
}
```

一些说明：

* label: 可以设置为调度平台的任务id
* envs: 环境变量，如果设置KMS，需要设置`MJ_KMS_TOKEN`
* enqueue_at: 设置任务的执行时间，可以设置为RFC3339格式，如`"2024-03-22T20:08:40+08:00"`，请注意添加时区。对应的命令如：
  `pueue add -d "2024-03-28T19:15:00" "java --version"`

nats publish "pueued.worker.worker1" "java --version"

# 回调

当任务执行完毕后，需要回调任务调度，要告知相关的信息.

* 日志OSS上上传： 文件命名规范为： 日志文件：`namespace/date/task_label.log`, 状态：`namespace/date/task_label.json`
* 发送NATS消息： subject: `pueued.tasks.callback`, message为json

```json
{
  "id": 1,
  "command": "java --version",
  "result": "Success",
  "exit_code": 1,
  "group": "default",
  "label": "xxxx",
  "start": 1234234,
  "end": 234234234,
  "output": "10 lines",
  "log_path": "s3://bucket/namespace/2024-03-02/task-label.log",
  "log_size": 11234234
}
```

每天可以考虑将相关的状态信息形成一个大的csv文件，保存下来。

# 客户端访问

如果你需要使用pueue控制多个客户端，可以考虑创建多个配置项，然后连接到不同的pueued服务器上。
然后创建对应的alias，如`pueue-a`, `pueue-b`，链接到不同的服务器上。

详细请参考： https://github.com/Nukesor/pueue/wiki/Configuration

callback的命令行程序要包含以下功能：

* 上传文件到OSS：通过环境变量
* 调用NATS publish

请参考[callbacks.rs](pueue/src/daemon/callbacks.rs)的实现，添加对应的回调功能。
