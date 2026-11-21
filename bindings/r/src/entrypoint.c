// The package's entry point: registers the Rust routines with R.
void R_init_mztabm_extendr(void *dll);

void R_init_mztabm(void *dll) {
    R_init_mztabm_extendr(dll);
}
