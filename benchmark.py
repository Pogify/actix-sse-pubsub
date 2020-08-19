from sseclient import SSEClient
from time import sleep
from threading import Lock, Thread

class Counter(Thread):
    def __init__(self):
        super().__init__()
        self.daemon = True
        self.counter = 0
        self.connections = []
        self.lock = Lock()

    def increment(self, connection):
        with self.lock:
            self.counter += 1
            # self.connections.append(connection)
            print(self.counter, end='\r')

counter = Counter()
counter.start()

def task(counter):

    try:
        messages = SSEClient("http://localhost:8080/events/channel1")
        print("yee")
    except OSError:
        return
    for msg in messages:
        if str(msg) == "connected":
            counter.increment(messages)
        if "secret" in str(msg):
            counter.increment(messages)


threads = []
for _ in range(10):
    t = Thread(target=task, args=(counter,), daemon=True)
    t.start()
    threads.append(t)

try:
    for t in threads:
        t.join()
except KeyboardInterrupt:
    exit()

