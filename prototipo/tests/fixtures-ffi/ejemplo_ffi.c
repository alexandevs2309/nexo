/* Biblioteca de ejemplo para las pruebas de FFI de BASILISCO (ABI C). */
long long sumar(long long a, long long b) { return a + b; }
long long cuadrado(long long x) { return x * x; }
int es_par(long long x) { return x % 2 == 0; }
double media(double a, double b) { return (a + b) / 2.0; }
double area_circulo(double r) { return 3.14159265358979323846 * r * r; }
double mezcla(long long a, double b) { return a + b; }
const char *saludo(void) { return "hola ffi"; }
long long largo(const char *s) { long long n = 0; while (s[n]) n++; return n; }
void nada(void) {}
