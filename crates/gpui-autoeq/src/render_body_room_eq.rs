{
        let mut form = VStack::new().spacing(StackSpacing::Lg);
        let base_id = id.clone();

        let toggle_theme = ToggleTheme {
            checked_bg: theme.toggle_checked_bg,
            unchecked_bg: theme.toggle_unchecked_bg,
            knob: theme.toggle_knob,
            knob_on_checked: theme.card_bg,
            track_border: theme.border,
            label: theme.label_color,
            accent: theme.accent,
            accent_muted: theme.accent,
            success: theme.accent,
            border: theme.border,
            text_on_accent: theme.toggle_knob,
            text_muted: theme.text_muted,
            text_primary: theme.header_color,
            surface_hover: theme.toggle_unchecked_bg,
            background: theme.card_bg,
        };

        let is_fir = config.opt_mode == "fir" || config.opt_mode == "mixed";
        let is_iir = config.opt_mode == "iir" || config.opt_mode == "mixed";

        // ========================================
        // Section 2: Room Configuration
        // ========================================
        {
            let mut target_col = VStack::new().spacing(StackSpacing::Sm);
            let mut options_col = VStack::new().spacing(StackSpacing::Sm);

            // --- Target sub-section ---
            target_col = target_col.child(
                Text::new("TARGET")
                    .size(TextSize::Xs)
                    .weight(TextWeight::Semibold)
                    .color(theme.accent),
            );

            // Target Tilt
            let mut tilt_toggle = Toggle::new((base_id.clone(), "tilt-enabled"))
                .size(ToggleSize::Sm)
                .checked(config.use_target_tilt)
                .theme(toggle_theme.clone());

            if let Some(ref h) = on_use_target_tilt_change_rc {
                let h = h.clone();
                tilt_toggle = tilt_toggle.on_change(move |v, w, cx| h(v, w, cx));
            }

            target_col = target_col.child(
                HStack::new()
                    .justify(StackJustify::SpaceBetween)
                    .child(
                        Text::new("Target Tilt")
                            .size(TextSize::Xs)
                            .color(theme.label_color),
                    )
                    .child(tilt_toggle),
            );

            if config.use_target_tilt {
                let tilt_options: Vec<SelectOption> = TILT_TYPE_OPTIONS
                    .iter()
                    .map(|(val, lbl)| SelectOption::new(*val, *lbl))
                    .collect();

                let mut tilt_select = Select::new((base_id.clone(), "tilt-type"))
                    .options(tilt_options)
                    .selected(&config.tilt_type)
                    .is_open(ui_state.tilt_type_open)
                    .theme(theme.select_theme.clone());

                if let Some(ref h) = on_tilt_type_toggle_rc {
                    let h = h.clone();
                    tilt_select = tilt_select.on_toggle(move |open, w, cx| h(open, w, cx));
                }

                if let Some(ref h) = on_tilt_type_change_rc {
                    let h = h.clone();
                    tilt_select = tilt_select.on_change(move |val, w, cx| h(val.as_ref(), w, cx));
                }

                target_col = target_col.child(tilt_select);

                if config.tilt_type == "custom" || config.tilt_type == "harman" {
                    let mut slope_input = NumberInput::new((base_id.clone(), "tilt-slope"))
                        .value(config.tilt_slope)
                        .min(ParamLimits::TILT_SLOPE.min)
                        .max(ParamLimits::TILT_SLOPE.max)
                        .step(ParamLimits::TILT_SLOPE.step)
                        .decimals(1)
                        .label("Slope (dB/oct)")
                        .size(NumberInputSize::Sm)
                        .theme(theme.number_input_theme.clone());

                    if let Some(ref h) = on_tilt_slope_change_rc {
                        let h = h.clone();
                        slope_input = slope_input.on_change(move |v, w, cx| h(v, w, cx));
                    }

                    let mut ref_freq_input = NumberInput::new((base_id.clone(), "tilt-ref-freq"))
                        .value(config.tilt_reference_freq)
                        .min(20.0)
                        .max(20000.0)
                        .step(10.0)
                        .decimals(0)
                        .label("Ref Freq (Hz)")
                        .size(NumberInputSize::Sm)
                        .theme(theme.number_input_theme.clone());

                    if let Some(ref h) = on_tilt_reference_freq_change_rc {
                        let h = h.clone();
                        ref_freq_input = ref_freq_input.on_change(move |v, w, cx| h(v, w, cx));
                    }

                    target_col = target_col.child(
                        HStack::new()
                            .spacing(StackSpacing::Md)
                            .child(slope_input)
                            .child(ref_freq_input),
                    );

                    let mut shelf_db_input = NumberInput::new((base_id.clone(), "tilt-shelf-db"))
                        .value(config.tilt_bass_shelf_db)
                        .min(ParamLimits::BASS_SHELF.min)
                        .max(ParamLimits::BASS_SHELF.max)
                        .step(ParamLimits::BASS_SHELF.step)
                        .decimals(1)
                        .label("Bass Boost (dB)")
                        .size(NumberInputSize::Sm)
                        .theme(theme.number_input_theme.clone());

                    if let Some(ref h) = on_tilt_bass_shelf_db_change_rc {
                        let h = h.clone();
                        shelf_db_input = shelf_db_input.on_change(move |v, w, cx| h(v, w, cx));
                    }

                    let mut shelf_freq_input =
                        NumberInput::new((base_id.clone(), "tilt-shelf-freq"))
                            .value(config.tilt_bass_shelf_freq)
                            .min(20.0)
                            .max(1000.0)
                            .step(10.0)
                            .decimals(0)
                            .label("Shelf Freq (Hz)")
                            .size(NumberInputSize::Sm)
                            .theme(theme.number_input_theme.clone());

                    if let Some(ref h) = on_tilt_bass_shelf_freq_change_rc {
                        let h = h.clone();
                        shelf_freq_input = shelf_freq_input.on_change(move |v, w, cx| h(v, w, cx));
                    }

                    target_col = target_col.child(
                        HStack::new()
                            .spacing(StackSpacing::Md)
                            .child(shelf_db_input)
                            .child(shelf_freq_input),
                    );
                }
            }

            // --- Options sub-section ---
            options_col = options_col.child(
                Text::new("OPTIONS")
                    .size(TextSize::Xs)
                    .weight(TextWeight::Semibold)
                    .color(theme.accent),
            );

            // Psychoacoustic Smoothing
            let mut psycho_toggle = Toggle::new((base_id.clone(), "psychoacoustic"))
                .size(ToggleSize::Sm)
                .checked(config.psychoacoustic)
                .theme(toggle_theme.clone());

            if let Some(ref handler) = on_psychoacoustic_change_rc {
                let h = handler.clone();
                psycho_toggle = psycho_toggle.on_change(move |v, w, cx| h(v, w, cx));
            }

            options_col = options_col.child(
                HStack::new()
                    .spacing(StackSpacing::Md)
                    .justify(StackJustify::SpaceBetween)
                    .child(
                        VStack::new()
                            .spacing(StackSpacing::None)
                            .child(
                                Text::new("Psychoacoustic Smoothing")
                                    .size(TextSize::Xs)
                                    .color(theme.label_color),
                            )
                            .child(
                                Text::new("1/48 oct bass, 1/6 oct treble")
                                    .size(TextSize::Xs)
                                    .color(theme.description_color),
                            ),
                    )
                    .child(psycho_toggle),
            );

            // Asymmetric Loss
            let mut asymmetric_toggle = Toggle::new((base_id.clone(), "asymmetric-loss"))
                .size(ToggleSize::Sm)
                .checked(config.asymmetric_loss)
                .theme(toggle_theme.clone());

            if let Some(ref handler) = on_asymmetric_loss_change_rc {
                let h = handler.clone();
                asymmetric_toggle = asymmetric_toggle.on_change(move |v, w, cx| h(v, w, cx));
            }

            options_col = options_col.child(
                HStack::new()
                    .spacing(StackSpacing::Md)
                    .justify(StackJustify::SpaceBetween)
                    .child(
                        VStack::new()
                            .spacing(StackSpacing::None)
                            .child(
                                Text::new("Asymmetric Loss")
                                    .size(TextSize::Xs)
                                    .color(theme.label_color),
                            )
                            .child(
                                Text::new("Penalize peaks more than dips")
                                    .size(TextSize::Xs)
                                    .color(theme.description_color),
                            ),
                    )
                    .child(asymmetric_toggle),
            );

            // Excursion Protection
            let mut excursion_toggle = Toggle::new((base_id.clone(), "excursion-enabled"))
                .size(ToggleSize::Sm)
                .checked(config.use_excursion_protection)
                .theme(toggle_theme.clone());

            if let Some(ref h) = on_use_excursion_protection_change_rc {
                let h = h.clone();
                excursion_toggle = excursion_toggle.on_change(move |v, w, cx| h(v, w, cx));
            }

            options_col = options_col.child(
                HStack::new()
                    .justify(StackJustify::SpaceBetween)
                    .child(
                        Text::new("Excursion Protection")
                            .size(TextSize::Xs)
                            .color(theme.label_color),
                    )
                    .child(excursion_toggle),
            );

            if config.use_excursion_protection {
                let mut auto_f3_toggle = Toggle::new((base_id.clone(), "excursion-auto-f3"))
                    .size(ToggleSize::Sm)
                    .checked(config.excursion_auto_detect_f3)
                    .theme(toggle_theme.clone());

                if let Some(ref h) = on_excursion_auto_detect_f3_change_rc {
                    let h = h.clone();
                    auto_f3_toggle = auto_f3_toggle.on_change(move |v, w, cx| h(v, w, cx));
                }

                options_col = options_col.child(
                    HStack::new()
                        .justify(StackJustify::SpaceBetween)
                        .child(
                            Text::new("Auto-detect F3")
                                .size(TextSize::Xs)
                                .color(theme.label_color),
                        )
                        .child(auto_f3_toggle),
                );

                if !config.excursion_auto_detect_f3 {
                    let mut f3_input = NumberInput::new((base_id.clone(), "excursion-manual-f3"))
                        .value(config.excursion_manual_f3)
                        .min(10.0)
                        .max(500.0)
                        .step(1.0)
                        .decimals(0)
                        .label("Manual F3 (Hz)")
                        .size(NumberInputSize::Sm)
                        .theme(theme.number_input_theme.clone());

                    if let Some(ref h) = on_excursion_manual_f3_change_rc {
                        let h = h.clone();
                        f3_input = f3_input.on_change(move |v, w, cx| h(v, w, cx));
                    }
                    options_col = options_col.child(f3_input);
                }

                let hp_options: Vec<SelectOption> = HIGHPASS_TYPE_OPTIONS
                    .iter()
                    .map(|(val, lbl)| SelectOption::new(*val, *lbl))
                    .collect();

                let mut hp_select = Select::new((base_id.clone(), "excursion-hp-type"))
                    .options(hp_options)
                    .selected(&config.excursion_filter_type)
                    .is_open(ui_state.excursion_filter_type_open)
                    .theme(theme.select_theme.clone());

                if let Some(ref h) = on_excursion_filter_type_toggle_rc {
                    let h = h.clone();
                    hp_select = hp_select.on_toggle(move |open, w, cx| h(open, w, cx));
                }

                if let Some(ref h) = on_excursion_filter_type_change_rc {
                    let h = h.clone();
                    hp_select = hp_select.on_change(move |val, w, cx| h(val.as_ref(), w, cx));
                }

                let mut order_input = NumberInput::new((base_id.clone(), "excursion-order"))
                    .value(config.excursion_filter_order as f64)
                    .min(2.0)
                    .max(8.0)
                    .step(2.0)
                    .decimals(0)
                    .label("Order")
                    .size(NumberInputSize::Sm)
                    .theme(theme.number_input_theme.clone());

                if let Some(ref h) = on_excursion_filter_order_change_rc {
                    let h = h.clone();
                    order_input =
                        order_input.on_change(move |v, w, cx| h(v.round() as usize, w, cx));
                }

                options_col = options_col.child(
                    HStack::new()
                        .spacing(StackSpacing::Md)
                        .child(hp_select)
                        .child(order_input),
                );

                let mut margin_input = NumberInput::new((base_id.clone(), "excursion-margin"))
                    .value(config.excursion_margin_octaves)
                    .min(0.0)
                    .max(1.0)
                    .step(0.05)
                    .decimals(2)
                    .label("Safety Margin (oct)")
                    .size(NumberInputSize::Sm)
                    .theme(theme.number_input_theme.clone());

                if let Some(ref h) = on_excursion_margin_octaves_change_rc {
                    let h = h.clone();
                    margin_input = margin_input.on_change(move |v, w, cx| h(v, w, cx));
                }

                options_col = options_col.child(margin_input);
            }

            // Schroeder Split
            let mut schroeder_toggle = Toggle::new((base_id.clone(), "schroeder-enabled"))
                .size(ToggleSize::Sm)
                .checked(config.use_schroeder_split)
                .theme(toggle_theme.clone());

            if let Some(ref h) = on_use_schroeder_split_change_rc {
                let h = h.clone();
                schroeder_toggle = schroeder_toggle.on_change(move |v, w, cx| h(v, w, cx));
            }

            options_col = options_col.child(
                HStack::new()
                    .justify(StackJustify::SpaceBetween)
                    .child(
                        Text::new("Schroeder Split")
                            .size(TextSize::Xs)
                            .color(theme.label_color),
                    )
                    .child(schroeder_toggle),
            );

            if config.use_schroeder_split {
                let mut s_freq_input = NumberInput::new((base_id.clone(), "schroeder-freq"))
                    .value(config.schroeder_freq)
                    .min(ParamLimits::SCHROEDER_FREQ.min)
                    .max(ParamLimits::SCHROEDER_FREQ.max)
                    .step(ParamLimits::SCHROEDER_FREQ.step)
                    .decimals(0)
                    .label("Split Freq (Hz)")
                    .size(NumberInputSize::Sm)
                    .theme(theme.number_input_theme.clone());

                if let Some(ref h) = on_schroeder_freq_change_rc {
                    let h = h.clone();
                    s_freq_input = s_freq_input.on_change(move |v, w, cx| h(v, w, cx));
                }

                options_col = options_col.child(s_freq_input);

                let mut low_q_input = NumberInput::new((base_id.clone(), "schroeder-low-q"))
                    .value(config.schroeder_low_max_q)
                    .min(1.0)
                    .max(20.0)
                    .step(0.5)
                    .decimals(1)
                    .label("LF Max Q")
                    .size(NumberInputSize::Sm)
                    .theme(theme.number_input_theme.clone());

                if let Some(ref h) = on_schroeder_low_max_q_change_rc {
                    let h = h.clone();
                    low_q_input = low_q_input.on_change(move |v, w, cx| h(v, w, cx));
                }

                let mut high_q_input = NumberInput::new((base_id.clone(), "schroeder-high-q"))
                    .value(config.schroeder_high_max_q)
                    .min(0.5)
                    .max(5.0)
                    .step(0.1)
                    .decimals(1)
                    .label("HF Max Q")
                    .size(NumberInputSize::Sm)
                    .theme(theme.number_input_theme.clone());

                if let Some(ref h) = on_schroeder_high_max_q_change_rc {
                    let h = h.clone();
                    high_q_input = high_q_input.on_change(move |v, w, cx| h(v, w, cx));
                }

                options_col = options_col.child(
                    HStack::new()
                        .spacing(StackSpacing::Md)
                        .child(low_q_input)
                        .child(high_q_input),
                );

                let mut boost_toggle = Toggle::new((base_id.clone(), "schroeder-boost"))
                    .size(ToggleSize::Sm)
                    .checked(config.schroeder_low_allow_boost)
                    .theme(toggle_theme.clone());

                if let Some(ref h) = on_schroeder_low_allow_boost_change_rc {
                    let h = h.clone();
                    boost_toggle = boost_toggle.on_change(move |v, w, cx| h(v, w, cx));
                }

                let mut shelve_toggle = Toggle::new((base_id.clone(), "schroeder-shelve"))
                    .size(ToggleSize::Sm)
                    .checked(config.schroeder_high_shelving_only)
                    .theme(toggle_theme.clone());

                if let Some(ref h) = on_schroeder_high_shelving_only_change_rc {
                    let h = h.clone();
                    shelve_toggle = shelve_toggle.on_change(move |v, w, cx| h(v, w, cx));
                }

                options_col = options_col.child(
                    HStack::new()
                        .justify(StackJustify::SpaceBetween)
                        .child(
                            Text::new("Allow LF Boost")
                                .size(TextSize::Xs)
                                .color(theme.label_color),
                        )
                        .child(boost_toggle),
                );

                options_col = options_col.child(
                    HStack::new()
                        .justify(StackJustify::SpaceBetween)
                        .child(
                            Text::new("HF Shelving Only")
                                .size(TextSize::Xs)
                                .color(theme.label_color),
                        )
                        .child(shelve_toggle),
                );
            }

            // Allow Delay
            let mut delay_toggle = Toggle::new((base_id.clone(), "allow-delay"))
                .size(ToggleSize::Sm)
                .checked(config.allow_delay)
                .theme(toggle_theme.clone());

            if let Some(ref h) = on_allow_delay_change_rc {
                let h = h.clone();
                delay_toggle = delay_toggle.on_change(move |v, w, cx| h(v, w, cx));
            }

            options_col = options_col.child(
                HStack::new()
                    .justify(StackJustify::SpaceBetween)
                    .child(
                        VStack::new()
                            .spacing(StackSpacing::None)
                            .child(
                                Text::new("Allow Delay")
                                    .size(TextSize::Xs)
                                    .color(theme.label_color),
                            )
                            .child(
                                Text::new("Enable inter-speaker time alignment")
                                    .size(TextSize::Xs)
                                    .color(theme.description_color),
                            ),
                    )
                    .child(delay_toggle),
            );

            // Broadband Target Matching
            let mut broadband_toggle =
                Toggle::new((base_id.clone(), "broadband-target-matching"))
                    .size(ToggleSize::Sm)
                    .checked(config.broadband_target_matching)
                    .theme(toggle_theme.clone());

            if let Some(ref h) = on_broadband_target_matching_change_rc {
                let h = h.clone();
                broadband_toggle = broadband_toggle.on_change(move |v, w, cx| h(v, w, cx));
            }

            options_col = options_col.child(
                HStack::new()
                    .justify(StackJustify::SpaceBetween)
                    .child(
                        VStack::new()
                            .spacing(StackSpacing::None)
                            .child(
                                Text::new("Broadband Target Matching")
                                    .size(TextSize::Xs)
                                    .color(theme.label_color),
                            )
                            .child(
                                Text::new("Shelf filters for broad tonal balance")
                                    .size(TextSize::Xs)
                                    .color(theme.description_color),
                            ),
                    )
                    .child(broadband_toggle),
            );

            // Group Delay Optimization
            let mut gd_toggle = Toggle::new((base_id.clone(), "gd-opt-enabled"))
                .size(ToggleSize::Sm)
                .checked(config.gd_opt_enabled)
                .theme(toggle_theme.clone());

            if let Some(ref h) = on_gd_opt_enabled_change_rc {
                let h = h.clone();
                gd_toggle = gd_toggle.on_change(move |v, w, cx| h(v, w, cx));
            }

            options_col = options_col.child(
                HStack::new()
                    .justify(StackJustify::SpaceBetween)
                    .child(
                        VStack::new()
                            .spacing(StackSpacing::None)
                            .child(
                                Text::new("Group Delay Optimization")
                                    .size(TextSize::Xs)
                                    .color(theme.label_color),
                            )
                            .child(
                                Text::new("Align group delay at crossover")
                                    .size(TextSize::Xs)
                                    .color(theme.description_color),
                            ),
                    )
                    .child(gd_toggle),
            );

            if config.gd_opt_enabled {
                let mut gd_target_input = NumberInput::new((base_id.clone(), "gd-target-ms"))
                    .value(config.gd_opt_target_ms)
                    .min(ParamLimits::GD_TARGET_MS.min)
                    .max(ParamLimits::GD_TARGET_MS.max)
                    .step(ParamLimits::GD_TARGET_MS.step)
                    .decimals(1)
                    .label("Target (ms)")
                    .size(NumberInputSize::Sm)
                    .width(120.0)
                    .disabled(disabled)
                    .theme(theme.number_input_theme.clone());

                if let Some(ref h) = on_gd_opt_target_ms_change_rc {
                    let h = h.clone();
                    gd_target_input = gd_target_input.on_change(move |v, w, cx| h(v, w, cx));
                }

                options_col = options_col.child(gd_target_input);
            }

            // Voice of God
            let mut vog_toggle = Toggle::new((base_id.clone(), "vog-enabled"))
                .size(ToggleSize::Sm)
                .checked(config.vog_enabled)
                .theme(toggle_theme.clone());

            if let Some(ref h) = on_vog_enabled_change_rc {
                let h = h.clone();
                vog_toggle = vog_toggle.on_change(move |v, w, cx| h(v, w, cx));
            }

            options_col = options_col.child(
                HStack::new()
                    .justify(StackJustify::SpaceBetween)
                    .child(
                        VStack::new()
                            .spacing(StackSpacing::None)
                            .child(
                                Text::new("Voice of God")
                                    .size(TextSize::Xs)
                                    .color(theme.label_color),
                            )
                            .child(
                                Text::new("Timbre matching across channels")
                                    .size(TextSize::Xs)
                                    .color(theme.description_color),
                            ),
                    )
                    .child(vog_toggle),
            );

            if config.vog_enabled {
                let ref_channel_options: Vec<SelectOption> = ["C", "L", "R"]
                    .iter()
                    .map(|ch| SelectOption::new(*ch, *ch))
                    .collect();

                let mut ref_select = Select::new((base_id.clone(), "vog-ref-channel"))
                    .label("Reference Channel")
                    .options(ref_channel_options)
                    .selected(&config.vog_reference_channel)
                    .is_open(ui_state.vog_reference_channel_open)
                    .disabled(disabled)
                    .theme(theme.select_theme.clone());

                if let Some(ref h) = on_vog_reference_channel_toggle_rc {
                    let h = h.clone();
                    ref_select = ref_select.on_toggle(move |open, w, cx| h(open, w, cx));
                }

                if let Some(ref h) = on_vog_reference_channel_change_rc {
                    let h = h.clone();
                    ref_select = ref_select.on_change(move |val, w, cx| h(val.as_ref(), w, cx));
                }

                options_col = options_col.child(ref_select);
            }

            // Phase Alignment (hidden for IIR mode)
            if !hide_phase_alignment {
                let mut phase_toggle = Toggle::new((base_id.clone(), "phase-enabled"))
                    .size(ToggleSize::Sm)
                    .checked(config.use_phase_alignment)
                    .theme(toggle_theme.clone());

                if let Some(ref h) = on_use_phase_alignment_change_rc {
                    let h = h.clone();
                    phase_toggle = phase_toggle.on_change(move |v, w, cx| h(v, w, cx));
                }

                options_col = options_col.child(
                    HStack::new()
                        .justify(StackJustify::SpaceBetween)
                        .child(
                            Text::new("Phase Alignment")
                                .size(TextSize::Xs)
                                .color(theme.label_color),
                        )
                        .child(phase_toggle),
                );

                if config.use_phase_alignment {
                    let mut min_freq_input = NumberInput::new((base_id.clone(), "phase-min-freq"))
                        .value(config.phase_min_freq)
                        .min(20.0)
                        .max(1000.0)
                        .step(1.0)
                        .decimals(0)
                        .label("Min Freq (Hz)")
                        .size(NumberInputSize::Sm)
                        .theme(theme.number_input_theme.clone());

                    if let Some(ref h) = on_phase_min_freq_change_rc {
                        let h = h.clone();
                        min_freq_input = min_freq_input.on_change(move |v, w, cx| h(v, w, cx));
                    }

                    let mut max_freq_input = NumberInput::new((base_id.clone(), "phase-max-freq"))
                        .value(config.phase_max_freq)
                        .min(20.0)
                        .max(1000.0)
                        .step(1.0)
                        .decimals(0)
                        .label("Max Freq (Hz)")
                        .size(NumberInputSize::Sm)
                        .theme(theme.number_input_theme.clone());

                    if let Some(ref h) = on_phase_max_freq_change_rc {
                        let h = h.clone();
                        max_freq_input = max_freq_input.on_change(move |v, w, cx| h(v, w, cx));
                    }

                    options_col = options_col.child(
                        HStack::new()
                            .spacing(StackSpacing::Md)
                            .child(min_freq_input)
                            .child(max_freq_input),
                    );

                    let mut polarity_toggle = Toggle::new((base_id.clone(), "phase-polarity"))
                        .size(ToggleSize::Sm)
                        .checked(config.phase_optimize_polarity)
                        .theme(toggle_theme.clone());

                    if let Some(ref h) = on_phase_optimize_polarity_change_rc {
                        let h = h.clone();
                        polarity_toggle = polarity_toggle.on_change(move |v, w, cx| h(v, w, cx));
                    }

                    options_col = options_col.child(
                        HStack::new()
                            .justify(StackJustify::SpaceBetween)
                            .child(
                                Text::new("Optimize Polarity")
                                    .size(TextSize::Xs)
                                    .color(theme.label_color),
                            )
                            .child(polarity_toggle),
                    );

                    let mut p_max_delay = NumberInput::new((base_id.clone(), "phase-max-delay"))
                        .value(config.phase_max_delay_ms)
                        .min(ParamLimits::DELAY_MS.min)
                        .max(ParamLimits::DELAY_MS.max)
                        .step(ParamLimits::DELAY_MS.step)
                        .decimals(1)
                        .label("Max Delay (ms)")
                        .size(NumberInputSize::Sm)
                        .theme(theme.number_input_theme.clone());

                    if let Some(ref h) = on_phase_max_delay_ms_change_rc {
                        let h = h.clone();
                        p_max_delay = p_max_delay.on_change(move |v, w, cx| h(v, w, cx));
                    }
                    options_col = options_col.child(p_max_delay);
                }
            }

            // Multi-Seat
            if !hide_multi_seat {
                let mut multi_seat_toggle = Toggle::new((base_id.clone(), "multi-seat-enabled"))
                    .size(ToggleSize::Sm)
                    .checked(config.use_multi_seat)
                    .theme(toggle_theme.clone());

                if let Some(ref h) = on_use_multi_seat_change_rc {
                    let h = h.clone();
                    multi_seat_toggle = multi_seat_toggle.on_change(move |v, w, cx| h(v, w, cx));
                }

                options_col = options_col.child(
                    HStack::new()
                        .justify(StackJustify::SpaceBetween)
                        .child(
                            Text::new("Multi-Seat Optimization")
                                .size(TextSize::Xs)
                                .color(theme.label_color),
                        )
                        .child(multi_seat_toggle),
                );

                if config.use_multi_seat {
                    let strategy_options: Vec<SelectOption> = MULTI_SEAT_STRATEGY_OPTIONS
                        .iter()
                        .map(|(val, lbl)| SelectOption::new(*val, *lbl))
                        .collect();

                    let mut strategy_select = Select::new((base_id.clone(), "multi-seat-strategy"))
                        .options(strategy_options)
                        .selected(&config.multi_seat_strategy)
                        .is_open(ui_state.multi_seat_strategy_open)
                        .theme(theme.select_theme.clone());

                    if let Some(ref h) = on_multi_seat_strategy_toggle_rc {
                        let h = h.clone();
                        strategy_select = strategy_select.on_toggle(move |open, w, cx| h(open, w, cx));
                    }

                    if let Some(ref h) = on_multi_seat_strategy_change_rc {
                        let h = h.clone();
                        strategy_select =
                            strategy_select.on_change(move |val, w, cx| h(val.as_ref(), w, cx));
                    }

                    options_col = options_col.child(strategy_select);

                    if config.multi_seat_strategy == "primary" {
                        let mut primary_seat_input =
                            NumberInput::new((base_id.clone(), "multi-seat-primary"))
                                .value(config.multi_seat_primary_seat as f64)
                                .min(0.0)
                                .max(16.0)
                                .step(1.0)
                                .decimals(0)
                                .label("Primary Seat")
                                .size(NumberInputSize::Sm)
                                .theme(theme.number_input_theme.clone());

                        if let Some(ref h) = on_multi_seat_primary_seat_change_rc {
                            let h = h.clone();
                            primary_seat_input = primary_seat_input
                                .on_change(move |v, w, cx| h(v.round() as usize, w, cx));
                        }
                        options_col = options_col.child(primary_seat_input);
                    }

                    let mut dev_input = NumberInput::new((base_id.clone(), "multi-seat-max-dev"))
                        .value(config.multi_seat_max_deviation_db)
                        .min(1.0)
                        .max(12.0)
                        .step(0.5)
                        .decimals(1)
                        .label("Max Deviation (dB)")
                        .size(NumberInputSize::Sm)
                        .theme(theme.number_input_theme.clone());

                    if let Some(ref h) = on_multi_seat_max_deviation_db_change_rc {
                        let h = h.clone();
                        dev_input = dev_input.on_change(move |v, w, cx| h(v, w, cx));
                    }
                    options_col = options_col.child(dev_input);
                }
            }

            // Assemble Room Configuration card
            let mut section = VStack::new().spacing(StackSpacing::Sm);

            // Header
            section = section.child(
                VStack::new()
                    .spacing(StackSpacing::None)
                    .child(
                        Text::new("Room Configuration")
                            .size(TextSize::Sm)
                            .weight(TextWeight::Semibold)
                            .color(theme.header_color),
                    )
                    .child(
                        Text::new("Target curve and room correction options")
                            .size(TextSize::Xs)
                            .color(theme.description_color),
                    ),
            );

            if available_width > 700.0 {
                section = section.child(
                    HStack::new()
                        .spacing(StackSpacing::Lg)
                        .child(div().flex_1().child(target_col))
                        .child(div().flex_1().child(options_col)),
                );
            } else {
                section = section.child(target_col).child(options_col);
            }

            form = form.child(Card::new().content(section));
        }

        // ========================================
        // Section 3: Optimiser Configuration
        // ========================================
        {
            let mut section = VStack::new().spacing(StackSpacing::Sm);

            // Header
            section = section.child(
                VStack::new()
                    .spacing(StackSpacing::None)
                    .child(
                        Text::new("Optimiser Configuration")
                            .size(TextSize::Sm)
                            .weight(TextWeight::Semibold)
                            .color(theme.header_color),
                    )
                    .child(
                        Text::new("EQ design and optimisation algorithm settings")
                            .size(TextSize::Xs)
                            .color(theme.description_color),
                    ),
            );

            // Two-column layout: EQ Design (left) + Optimisation (right)
            let mut left_col = VStack::new().spacing(StackSpacing::Sm);
            let mut right_col = VStack::new().spacing(StackSpacing::Sm);

            // Left column: EQ Design params
            left_col = left_col.child(
                Text::new("EQ DESIGN")
                    .size(TextSize::Xs)
                    .weight(TextWeight::Semibold)
                    .color(theme.accent),
            );

            // Filters (IIR)
            if is_iir {
                let mut num_filters_input = NumberInput::new((base_id.clone(), "num-filters"))
                    .value(config.num_filters as f64)
                    .min(ParamLimits::NUM_FILTERS.min)
                    .max(ParamLimits::NUM_FILTERS.max)
                    .step(ParamLimits::NUM_FILTERS.step)
                    .decimals(0)
                    .label("Filters")
                    .size(NumberInputSize::Sm)
                    .width(100.0)
                    .disabled(disabled)
                    .theme(theme.number_input_theme.clone());

                if let Some(ref handler) = on_num_filters_change_rc {
                    let h = handler.clone();
                    num_filters_input =
                        num_filters_input.on_change(move |v, w, cx| h(v.round() as usize, w, cx));
                }

                if !hide_sample_rate {
                    let mut sample_rate_input =
                        NumberInput::new((base_id.clone(), "sample-rate"))
                            .value(config.sample_rate as f64)
                            .min(ParamLimits::SAMPLE_RATE.min)
                            .max(ParamLimits::SAMPLE_RATE.max)
                            .step(ParamLimits::SAMPLE_RATE.step)
                            .decimals(0)
                            .label("Sample Rate")
                            .size(NumberInputSize::Sm)
                            .width(100.0)
                            .disabled(disabled)
                            .theme(theme.number_input_theme.clone());

                    if let Some(ref handler) = on_sample_rate_change_rc {
                        let h = handler.clone();
                        sample_rate_input = sample_rate_input
                            .on_change(move |v, w, cx| h(v.round() as usize, w, cx));
                    }

                    left_col = left_col.child(
                        HStack::new()
                            .spacing(StackSpacing::Md)
                            .child(num_filters_input)
                            .child(sample_rate_input),
                    );
                } else {
                    left_col = left_col.child(num_filters_input);
                }
            } else if !hide_sample_rate {
                let mut sample_rate_input = NumberInput::new((base_id.clone(), "sample-rate"))
                    .value(config.sample_rate as f64)
                    .min(ParamLimits::SAMPLE_RATE.min)
                    .max(ParamLimits::SAMPLE_RATE.max)
                    .step(ParamLimits::SAMPLE_RATE.step)
                    .decimals(0)
                    .label("Sample Rate")
                    .size(NumberInputSize::Sm)
                    .width(100.0)
                    .disabled(disabled)
                    .theme(theme.number_input_theme.clone());

                if let Some(ref handler) = on_sample_rate_change_rc {
                    let h = handler.clone();
                    sample_rate_input =
                        sample_rate_input.on_change(move |v, w, cx| h(v.round() as usize, w, cx));
                }

                left_col = left_col.child(sample_rate_input);
            }

            // FIR Taps and Phase (when FIR or mixed)
            if is_fir {
                let mut fir_taps_input = NumberInput::new((base_id.clone(), "fir-taps"))
                    .value(config.fir_taps as f64)
                    .min(ParamLimits::FIR_TAPS.min)
                    .max(ParamLimits::FIR_TAPS.max)
                    .step(ParamLimits::FIR_TAPS.step)
                    .decimals(0)
                    .label("FIR Taps")
                    .size(NumberInputSize::Sm)
                    .width(100.0)
                    .disabled(disabled)
                    .theme(theme.number_input_theme.clone());

                if let Some(ref handler) = on_fir_taps_change_rc {
                    let h = handler.clone();
                    fir_taps_input =
                        fir_taps_input.on_change(move |v, w, cx| h(v.round() as usize, w, cx));
                }

                let fir_phase_options: Vec<SelectOption> = FIR_PHASE_OPTIONS
                    .iter()
                    .map(|(val, lbl)| SelectOption::new(*val, *lbl))
                    .collect();

                let mut fir_phase_select = Select::new((base_id.clone(), "fir-phase"))
                    .label("Phase")
                    .options(fir_phase_options)
                    .selected(&config.fir_phase)
                    .is_open(ui_state.fir_phase_open)
                    .disabled(disabled)
                    .theme(theme.select_theme.clone());

                if let Some(ref handler) = on_fir_phase_toggle_rc {
                    let h = handler.clone();
                    fir_phase_select =
                        fir_phase_select.on_toggle(move |open, w, cx| h(open, w, cx));
                }

                if let Some(ref handler) = on_fir_phase_change_rc {
                    let h = handler.clone();
                    fir_phase_select =
                        fir_phase_select.on_change(move |value, w, cx| h(value.as_ref(), w, cx));
                }

                left_col = left_col.child(
                    HStack::new()
                        .spacing(StackSpacing::Md)
                        .child(fir_taps_input)
                        .child(fir_phase_select),
                );
            }

            // Mixed mode config
            if config.opt_mode == "mixed" {
                let mut mixed_freq_input =
                    NumberInput::new((base_id.clone(), "mixed-crossover-freq"))
                        .value(config.mixed_crossover_freq)
                        .min(ParamLimits::MIXED_CROSSOVER_FREQ.min)
                        .max(ParamLimits::MIXED_CROSSOVER_FREQ.max)
                        .step(ParamLimits::MIXED_CROSSOVER_FREQ.step)
                        .decimals(0)
                        .label("Crossover Freq (Hz)")
                        .size(NumberInputSize::Sm)
                        .width(140.0)
                        .disabled(disabled)
                        .theme(theme.number_input_theme.clone());

                if let Some(ref handler) = on_mixed_crossover_freq_change_rc {
                    let h = handler.clone();
                    mixed_freq_input = mixed_freq_input.on_change(move |v, w, cx| h(v, w, cx));
                }

                let mixed_type_options: Vec<SelectOption> = MIXED_CROSSOVER_TYPE_OPTIONS
                    .iter()
                    .map(|(val, lbl)| SelectOption::new(*val, *lbl))
                    .collect();

                let mut mixed_type_select =
                    Select::new((base_id.clone(), "mixed-crossover-type"))
                        .label("Crossover Type")
                        .options(mixed_type_options)
                        .selected(&config.mixed_crossover_type)
                        .is_open(ui_state.mixed_crossover_type_open)
                        .disabled(disabled)
                        .theme(theme.select_theme.clone());

                if let Some(ref handler) = on_mixed_crossover_type_toggle_rc {
                    let h = handler.clone();
                    mixed_type_select =
                        mixed_type_select.on_toggle(move |open, w, cx| h(open, w, cx));
                }

                if let Some(ref handler) = on_mixed_crossover_type_change_rc {
                    let h = handler.clone();
                    mixed_type_select =
                        mixed_type_select.on_change(move |val, w, cx| h(val.as_ref(), w, cx));
                }

                let mixed_band_options: Vec<SelectOption> = MIXED_FIR_BAND_OPTIONS
                    .iter()
                    .map(|(val, lbl)| SelectOption::new(*val, *lbl))
                    .collect();

                let mut mixed_band_select = Select::new((base_id.clone(), "mixed-fir-band"))
                    .label("FIR Band")
                    .options(mixed_band_options)
                    .selected(&config.mixed_fir_band)
                    .is_open(ui_state.mixed_fir_band_open)
                    .disabled(disabled)
                    .theme(theme.select_theme.clone());

                if let Some(ref handler) = on_mixed_fir_band_toggle_rc {
                    let h = handler.clone();
                    mixed_band_select =
                        mixed_band_select.on_toggle(move |open, w, cx| h(open, w, cx));
                }

                if let Some(ref handler) = on_mixed_fir_band_change_rc {
                    let h = handler.clone();
                    mixed_band_select =
                        mixed_band_select.on_change(move |val, w, cx| h(val.as_ref(), w, cx));
                }

                left_col = left_col
                    .child(mixed_freq_input)
                    .child(mixed_type_select)
                    .child(mixed_band_select);
            }

            // dB Range
            let mut min_db_input = NumberInput::new((base_id.clone(), "min-db"))
                .value(config.min_db)
                .min(ParamLimits::DB.min)
                .max(ParamLimits::DB.max)
                .step(ParamLimits::DB.step)
                .decimals(1)
                .label("Min dB")
                .size(NumberInputSize::Sm)
                .width(100.0)
                .disabled(disabled)
                .theme(theme.number_input_theme.clone());

            if let Some(ref handler) = on_min_db_change_rc {
                let h = handler.clone();
                min_db_input = min_db_input.on_change(move |v, w, cx| h(v, w, cx));
            }

            let mut max_db_input = NumberInput::new((base_id.clone(), "max-db"))
                .value(config.max_db)
                .min(ParamLimits::DB.min)
                .max(ParamLimits::DB.max)
                .step(ParamLimits::DB.step)
                .decimals(1)
                .label("Max dB")
                .size(NumberInputSize::Sm)
                .width(100.0)
                .disabled(disabled)
                .theme(theme.number_input_theme.clone());

            if let Some(ref handler) = on_max_db_change_rc {
                let h = handler.clone();
                max_db_input = max_db_input.on_change(move |v, w, cx| h(v, w, cx));
            }

            left_col = left_col.child(
                HStack::new()
                    .spacing(StackSpacing::Md)
                    .child(min_db_input)
                    .child(max_db_input),
            );

            // Q Range (IIR only)
            if is_iir {
                let mut min_q_input = NumberInput::new((base_id.clone(), "min-q"))
                    .value(config.min_q)
                    .min(ParamLimits::Q.min)
                    .max(ParamLimits::Q.max)
                    .step(ParamLimits::Q.step)
                    .decimals(1)
                    .label("Min Q")
                    .size(NumberInputSize::Sm)
                    .width(100.0)
                    .disabled(disabled)
                    .theme(theme.number_input_theme.clone());

                if let Some(ref handler) = on_min_q_change_rc {
                    let h = handler.clone();
                    min_q_input = min_q_input.on_change(move |v, w, cx| h(v, w, cx));
                }

                let mut max_q_input = NumberInput::new((base_id.clone(), "max-q"))
                    .value(config.max_q)
                    .min(ParamLimits::Q.min)
                    .max(ParamLimits::Q.max)
                    .step(ParamLimits::Q.step)
                    .decimals(1)
                    .label("Max Q")
                    .size(NumberInputSize::Sm)
                    .width(100.0)
                    .disabled(disabled)
                    .theme(theme.number_input_theme.clone());

                if let Some(ref handler) = on_max_q_change_rc {
                    let h = handler.clone();
                    max_q_input = max_q_input.on_change(move |v, w, cx| h(v, w, cx));
                }

                left_col = left_col.child(
                    HStack::new()
                        .spacing(StackSpacing::Md)
                        .child(min_q_input)
                        .child(max_q_input),
                );
            }

            // Frequency Range
            let mut min_freq_input = NumberInput::new((base_id.clone(), "min-freq"))
                .value(config.min_freq)
                .min(ParamLimits::FREQUENCY.min)
                .max(ParamLimits::FREQUENCY.max)
                .step(ParamLimits::FREQUENCY.step)
                .decimals(0)
                .label("Min Freq")
                .size(NumberInputSize::Sm)
                .width(100.0)
                .disabled(disabled)
                .theme(theme.number_input_theme.clone());

            if let Some(ref handler) = on_min_freq_change_rc {
                let h = handler.clone();
                min_freq_input = min_freq_input.on_change(move |v, w, cx| h(v, w, cx));
            }

            let mut max_freq_input = NumberInput::new((base_id.clone(), "max-freq"))
                .value(config.max_freq)
                .min(ParamLimits::FREQUENCY.min)
                .max(ParamLimits::FREQUENCY.max)
                .step(ParamLimits::FREQUENCY.step)
                .decimals(0)
                .label("Max Freq")
                .size(NumberInputSize::Sm)
                .width(100.0)
                .disabled(disabled)
                .theme(theme.number_input_theme.clone());

            if let Some(ref handler) = on_max_freq_change_rc {
                let h = handler.clone();
                max_freq_input = max_freq_input.on_change(move |v, w, cx| h(v, w, cx));
            }

            left_col = left_col.child(
                HStack::new()
                    .spacing(StackSpacing::Md)
                    .child(min_freq_input)
                    .child(max_freq_input),
            );

            // PEQ Model (IIR only)
            if is_iir {
                let peq_model_options: Vec<SelectOption> = PEQ_MODEL_OPTIONS
                    .iter()
                    .map(|(val, lbl)| SelectOption::new(*val, *lbl))
                    .collect();

                let mut peq_model_select = Select::new((base_id.clone(), "peq-model"))
                    .label("PEQ Model")
                    .options(peq_model_options)
                    .selected(&config.peq_model)
                    .is_open(ui_state.peq_model_open)
                    .disabled(disabled)
                    .theme(theme.select_theme.clone());

                if let Some(ref handler) = on_peq_model_toggle_rc {
                    let h = handler.clone();
                    peq_model_select =
                        peq_model_select.on_toggle(move |open, w, cx| h(open, w, cx));
                }

                if let Some(ref handler) = on_peq_model_change_rc {
                    let h = handler.clone();
                    peq_model_select =
                        peq_model_select.on_change(move |value, w, cx| h(value.as_ref(), w, cx));
                }

                left_col = left_col.child(peq_model_select);
            }

            // Right column: Optimisation params
            right_col = right_col.child(
                Text::new("OPTIMISATION")
                    .size(TextSize::Xs)
                    .weight(TextWeight::Semibold)
                    .color(theme.accent),
            );

            // Algorithm
            let algo_options: Vec<SelectOption> = ALGORITHM_OPTIONS
                .iter()
                .map(|(val, lbl)| SelectOption::new(*val, *lbl))
                .collect();

            let mut algo_select = Select::new((base_id.clone(), "algo"))
                .label("Algorithm")
                .options(algo_options)
                .selected(&config.algo)
                .is_open(ui_state.algo_open)
                .disabled(disabled)
                .theme(theme.select_theme.clone());

            if let Some(ref handler) = on_algo_toggle_rc {
                let h = handler.clone();
                algo_select = algo_select.on_toggle(move |open, w, cx| h(open, w, cx));
            }

            if let Some(ref handler) = on_algo_change_rc {
                let h = handler.clone();
                algo_select = algo_select.on_change(move |value, w, cx| h(value.as_ref(), w, cx));
            }

            right_col = right_col.child(algo_select);

            // Population + Max Evals
            let mut population_input = NumberInput::new((base_id.clone(), "population"))
                .value(config.population as f64)
                .min(ParamLimits::POPULATION.min)
                .max(ParamLimits::POPULATION.max)
                .step(ParamLimits::POPULATION.step)
                .decimals(0)
                .label("Population")
                .size(NumberInputSize::Sm)
                .width(100.0)
                .disabled(disabled)
                .theme(theme.number_input_theme.clone());

            if let Some(ref handler) = on_population_change_rc {
                let h = handler.clone();
                population_input =
                    population_input.on_change(move |v, w, cx| h(v.round() as usize, w, cx));
            }

            let mut maxeval_input = NumberInput::new((base_id.clone(), "maxeval"))
                .value(config.maxeval as f64)
                .min(ParamLimits::MAXEVAL.min)
                .max(ParamLimits::MAXEVAL.max)
                .step(ParamLimits::MAXEVAL.step)
                .decimals(0)
                .label("Max Evals")
                .size(NumberInputSize::Sm)
                .width(100.0)
                .disabled(disabled)
                .theme(theme.number_input_theme.clone());

            if let Some(ref handler) = on_maxeval_change_rc {
                let h = handler.clone();
                maxeval_input =
                    maxeval_input.on_change(move |v, w, cx| h(v.round() as usize, w, cx));
            }

            right_col = right_col.child(
                HStack::new()
                    .spacing(StackSpacing::Md)
                    .child(population_input)
                    .child(maxeval_input),
            );

            // Local Refinement
            let mut refine_toggle = Toggle::new((base_id.clone(), "refine"))
                .size(ToggleSize::Sm)
                .checked(config.refine)
                .theme(toggle_theme.clone());

            if let Some(ref handler) = on_refine_change_rc {
                let h = handler.clone();
                refine_toggle = refine_toggle.on_change(move |v, w, cx| h(v, w, cx));
            }

            right_col = right_col.child(
                HStack::new()
                    .spacing(StackSpacing::Md)
                    .justify(StackJustify::SpaceBetween)
                    .child(
                        Text::new("Local Refinement")
                            .size(TextSize::Xs)
                            .color(theme.label_color),
                    )
                    .child(refine_toggle),
            );

            if config.refine {
                let local_algo_options: Vec<SelectOption> = LOCAL_ALGO_OPTIONS
                    .iter()
                    .map(|(val, lbl)| SelectOption::new(*val, *lbl))
                    .collect();

                let mut local_algo_select = Select::new((base_id.clone(), "local-algo"))
                    .label("Local Algo")
                    .options(local_algo_options)
                    .selected(&config.local_algo)
                    .is_open(ui_state.local_algo_open)
                    .disabled(disabled)
                    .theme(theme.select_theme.clone());

                if let Some(ref handler) = on_local_algo_toggle_rc {
                    let h = handler.clone();
                    local_algo_select =
                        local_algo_select.on_toggle(move |open, w, cx| h(open, w, cx));
                }

                if let Some(ref handler) = on_local_algo_change_rc {
                    let h = handler.clone();
                    local_algo_select =
                        local_algo_select.on_change(move |value, w, cx| h(value.as_ref(), w, cx));
                }

                right_col = right_col.child(local_algo_select);
            }

            section = section.child(
                HStack::new()
                    .spacing(StackSpacing::Lg)
                    .child(div().flex_1().child(left_col))
                    .child(div().flex_1().child(right_col)),
            );

            form = form.child(Card::new().content(section));
        }

        div().id(id).child(form)
}
