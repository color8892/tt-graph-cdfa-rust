// Minimal C++ Parallel Sections Demo for CDFA Diagnostics (Optimized/Resolved).
int v1 = 0;
int v2 = 0;

void tt_print(const int &) {}

void program1_simple() {
#pragma omp parallel sections
  {
#pragma omp section
    {
      v1 = 10; // Thread 1 writes to v1 (Write) -> Independent variable
    }
#pragma omp section
    {
      tt_print(v2); // Thread 2 reads from v2 (Read) -> Independent variable, no anomaly!
    }
  }
}
