// Minimal C++ Parallel Sections Demo for CDFA Diagnostics.
int v = 0;

void tt_print(const int &) {}

void program1_simple() {
#pragma omp parallel sections
  {
#pragma omp section
    {
      v = 10; // Thread 1 writes to v (Write)
    }
#pragma omp section
    {
      tt_print(v); // Thread 2 reads from v (Read) -> CCA: WriteRead anomaly!
    }
  }
}
