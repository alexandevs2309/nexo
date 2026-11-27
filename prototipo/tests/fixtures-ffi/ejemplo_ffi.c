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
const char *texto_nulo(void) { return 0; }
double retorna_nan(void) { return 0.0 / 0.0; }
double retorna_inf(void) { return 1.0 / 0.0; }
double retorna_decimal_grande(void) { return 1e300; }
long long id_entero(long long x) { return x; }
static char eco_buf[256];
const char *eco_texto(const char *s) {
    long long i = 0;
    while (s[i] && i < 255) { eco_buf[i] = s[i]; i++; }
    eco_buf[i] = 0;
    return eco_buf;
}
static long long contador = 0;
void contador_reset(void) { contador = 0; }
long long contador_inc(void) { contador++; return contador; }
long long contador_get(void) { return contador; }
