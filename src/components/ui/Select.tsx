import React from "react";
import SelectComponent from "react-select";
import CreatableSelect from "react-select/creatable";
import type {
  ActionMeta,
  Props as ReactSelectProps,
  SingleValue,
  StylesConfig,
} from "react-select";

export type SelectOption = {
  value: string;
  label: string;
  isDisabled?: boolean;
};

type BaseProps = {
  value: string | null;
  options: SelectOption[];
  placeholder?: string;
  disabled?: boolean;
  isLoading?: boolean;
  isClearable?: boolean;
  onChange: (value: string | null, action: ActionMeta<SelectOption>) => void;
  onBlur?: () => void;
  className?: string;
  formatCreateLabel?: (input: string) => string;
};

type CreatableProps = {
  isCreatable: true;
  onCreateOption: (value: string) => void;
};

type NonCreatableProps = {
  isCreatable?: false;
  onCreateOption?: never;
};

export type SelectProps = BaseProps & (CreatableProps | NonCreatableProps);

const hoverBackground = "color-mix(in srgb, var(--color-ink) 4%, transparent)";
const optionSelectedBg =
  "color-mix(in srgb, var(--color-accent) 12%, transparent)";
const optionFocusedBg =
  "color-mix(in srgb, var(--color-accent) 6%, transparent)";

const selectStyles: StylesConfig<SelectOption, false> = {
  control: (base, state) => ({
    ...base,
    minHeight: 36,
    borderRadius: 3,
    border: "none",
    borderBottom: `1px solid ${
      state.isFocused ? "var(--color-accent)" : "var(--color-rule)"
    }`,
    boxShadow: "none",
    backgroundColor: "transparent",
    fontSize: "13px",
    color: "var(--color-ink)",
    transition: "border-color 200ms ease-out",
    ":hover": {
      borderBottom:
        "1px solid color-mix(in srgb, var(--color-ink) 40%, transparent)",
      backgroundColor: hoverBackground,
    },
  }),
  valueContainer: (base) => ({
    ...base,
    paddingInline: 0,
    paddingBlock: 6,
  }),
  input: (base) => ({
    ...base,
    color: "var(--color-ink)",
  }),
  singleValue: (base) => ({
    ...base,
    color: "var(--color-ink)",
  }),
  indicatorSeparator: () => ({ display: "none" }),
  dropdownIndicator: (base, state) => ({
    ...base,
    padding: 4,
    color: state.isFocused ? "var(--color-accent)" : "var(--color-muted)",
    ":hover": {
      color: "var(--color-accent)",
    },
  }),
  clearIndicator: (base) => ({
    ...base,
    padding: 4,
    color: "var(--color-muted)",
    ":hover": {
      color: "var(--color-accent)",
    },
  }),
  menu: (provided) => ({
    ...provided,
    zIndex: 30,
    backgroundColor: "var(--color-paper)",
    color: "var(--color-ink)",
    border: "1px solid var(--color-rule)",
    borderRadius: 5,
    boxShadow: "0 8px 32px rgba(0, 0, 0, 0.18)",
    overflow: "hidden",
  }),
  menuList: (base) => ({
    ...base,
    padding: 4,
  }),
  option: (base, state) => ({
    ...base,
    fontSize: "13px",
    borderRadius: 3,
    backgroundColor: state.isSelected
      ? optionSelectedBg
      : state.isFocused
        ? optionFocusedBg
        : "transparent",
    color: state.isSelected ? "var(--color-accent)" : "var(--color-ink)",
    cursor: state.isDisabled ? "not-allowed" : base.cursor,
    opacity: state.isDisabled ? 0.4 : 1,
    transition: "background-color 150ms ease-out, color 150ms ease-out",
  }),
  placeholder: (base) => ({
    ...base,
    color: "var(--color-muted)",
    opacity: 0.7,
  }),
};

export const Select: React.FC<SelectProps> = React.memo(
  ({
    value,
    options,
    placeholder,
    disabled,
    isLoading,
    isClearable = true,
    onChange,
    onBlur,
    className = "",
    isCreatable,
    formatCreateLabel,
    onCreateOption,
  }) => {
    const selectValue = React.useMemo(() => {
      if (!value) return null;
      const existing = options.find((option) => option.value === value);
      if (existing) return existing;
      return { value, label: value, isDisabled: false };
    }, [value, options]);

    const handleChange = (
      option: SingleValue<SelectOption>,
      action: ActionMeta<SelectOption>,
    ) => {
      onChange(option?.value ?? null, action);
    };

    const sharedProps: Partial<ReactSelectProps<SelectOption, false>> = {
      className,
      classNamePrefix: "app-select",
      value: selectValue,
      options,
      onChange: handleChange,
      placeholder,
      isDisabled: disabled,
      isLoading,
      onBlur,
      isClearable,
      styles: selectStyles,
    };

    if (isCreatable) {
      return (
        <CreatableSelect<SelectOption, false>
          {...sharedProps}
          onCreateOption={onCreateOption}
          formatCreateLabel={formatCreateLabel}
        />
      );
    }

    return <SelectComponent<SelectOption, false> {...sharedProps} />;
  },
);

Select.displayName = "Select";
