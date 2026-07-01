// Paper Program 1 — C++ parsed by libclang from OpenMP parallel sections.
int v = 0;
int i = 0;

void tt_print(int &) {}
void tt_kill(int &) {}

void program1() {
#pragma omp parallel sections
  {
#pragma omp section
    {
      tt_print(v);
      v = 10;
      i = v;
      while (i < 20) {
        tt_print(v);
        i = i + 1;
      }
    }
#pragma omp section
    {
      tt_print(v);
      v = 1000;
      if (v % 2 == 0) {
        tt_print(v * 10);
        i = 10;
        tt_kill(v);
      } else {
        tt_print(v / 10);
        tt_kill(v);
      }
    }
  }
}