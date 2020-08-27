import asyncio
from sseclient import SSEClient

import logging
from itertools import count
from time import sleep
from threading import Lock, Thread

logging.basicConfig(filename="sse.log", level=logging.DEBUG)

class Counter:
    def __init__(self):
        # super().__init__()
        self.daemon = True
        self.counter = 0
        self._number_of_read = 0
        self._counter = count()
        self.lock = Lock()

    def increment(self):
        with self.lock:
            self.counter += 1
            print(self.counter, end='\r', flush=True)


counter = Counter()
# counter.start()


def task(counter):
    try:
        messages = SSEClient("http://localhost:8080/events/channel1")
        counter.increment()
    except OSError as e:
        logging.error(str(e))
        return
    for msg in messages:
        if "secret" in str(msg):
            counter.increment()


threads = []
for i in range(5000):
#     if i > 4000 and i % 1000 == 0:
#         sleep(1)
    t = Thread(target=task, args=(counter,), daemon=True)
    t.start()
    threads.append(t)

try:
    for t in threads:
        t.join()
except KeyboardInterrupt:
    exit()

