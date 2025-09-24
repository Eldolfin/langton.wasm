# Langton's ant

A parametrized langton's ant simulator written in rust for the browser.

I use this as part of my personal website which you can find at
[cv.eldolfin.top](https://cv.eldolfin.top)

## Try it

[![Demo](https://github.com/user-attachments/assets/69129da3-f31c-46ad-8c18-863c19a08726)](https://eldolfin.codeberg.page/langton.wasm/?alpha_retention=255&ant_color_brightness=1&ant_color_saturation=0&cell_border_size=1&cell_size=10&debug=&number_of_ants=1&start_x=0.5&start_y=0.5)

## Cool parameters examples

- [Many small ants](https://eldolfin.codeberg.page/langton.wasm/?alpha_retention=235&cell_size=5&final_speed=0.5&number_of_ants=400&speedup_frames=0&start_x=0.5&start_y=0.5)
- [3 trailing ants](https://eldolfin.codeberg.page/langton.wasm/?alpha_retention=255&final_speed=30&number_of_ants=3&speedup_frames=300&start_x=0.5&start_y=0.5&cell_size=4)
- [Angry ant](https://eldolfin.codeberg.page/langton.wasm/?alpha_retention=220&final_speed=200&number_of_ants=1&speedup_frames=0)
- [Flies](https://eldolfin.codeberg.page/langton.wasm/?alpha_retention=0&ant_color_brightness=0.3&ant_color_saturation=0&cell_border_size=0&cell_size=6&final_speed=1&number_of_ants=500&speed_ease-in_power=1&speedup_frames=120&start_x=0.5&start_y=0.5&white_color_blue=0&white_color_green=0&white_color_red=0)
- [Chaos](https://eldolfin.codeberg.page/langton.wasm/?alpha_retention=255&final_speed=40&number_of_ants=300&speedup_frames=600&start_x=0.5&start_y=0.5)
- [Small grid](https://eldolfin.codeberg.page/langton.wasm/?alpha_retention=254&ant_color_brightness=0.65&ant_color_saturation=1&cell_border_size=0&cell_size=5&final_speed=25&number_of_ants=4&speed_ease-in_power=7&speedup_frames=1200&start_x=0.5&start_y=0.5&white_color_blue=227&white_color_green=227&white_color_red=227)
- [1px grid benchmark](https://eldolfin.codeberg.page/langton.wasm/?alpha_retention=255&ant_color_brightness=0&ant_color_saturation=0.5&cell_border_size=0&cell_size=1&debug=&final_speed=5000&number_of_ants=1&speedup_frames=0&white_color_blue=255&white_color_green=255&white_color_red=255)