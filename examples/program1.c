// Paper Program 1 in a C-like subset (parallel/branch are TT Graph extensions).
parallel And1 {
  branch B1 {
    print(v);
    v = 10;
    i = v;
    while (i < 20) {
      print(v);
      i = i + 1;
    }
  }
  branch B2 {
    print(v);
    v = 1000;
    if (v % 2 == 0) {
      print(v * 10);
      i = 10;
      kill(v);
    } else {
      print(v / 10);
      kill(v);
    }
  }
}