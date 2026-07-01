// Paper Program 1 — plain C++17: std::thread parallelism, printf I/O, no TT/OpenMP markers.
#include <cstdio>
#include <cstdlib>
#include <thread>

int v = 0;
int i = 0;

void worker_b1() {
  printf("%d", v);
  v = 10;
  i = v;
  while (i < 20) {
    printf("%d", v);
    i = i + 1;
  }
}

void worker_b2() {
  printf("%d", v);
  v = 1000;
  if (v % 2 == 0) {
    printf("%d", v * 10);
    i = 10;
    free((void *)&v);
  } else {
    printf("%d", v / 10);
    free((void *)&v);
  }
}

void program1() {
  std::thread t1(worker_b1);
  std::thread t2(worker_b2);
  t1.join();
  t2.join();
}