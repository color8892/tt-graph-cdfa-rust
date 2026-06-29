// Legacy TT Graph scaffolding via `#pragma tt` (still supported).
int v = 0;
int i = 0;

void tt_print(int &) {}
void tt_kill(int &) {}

#pragma tt parallel And1

#pragma tt branch B1
void tt_B1() {
  tt_print(v);
  v = 10;
  i = v;
  while (i < 20) {
    tt_print(v);
    i = i + 1;
  }
}

#pragma tt branch B2
void tt_B2() {
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