import re
import csv

input_file = r"C:\Users\hrag\Sync\Programming\rust\pst_reader\src\dev\[MS-OXPROPS]-250520.txt"
output_file = r"C:\Users\hrag\Sync\Programming\rust\pst_reader\src\dev\[MS-OXPROPS]-250520.tsv"

# Fields we want in the output
fields = [
    "Canonical name",
    "Description",
    "Property set",
    "Property ID",
    "Property long ID (LID)",
    "Data type",
    "Area",
    "Defining reference",
    "Alternate names"
]

def parse_entries(text):
    # Split on lines like "2.1   Something"
    entries = re.split(r'\n(?=\d+\.\d+\t)', text)

    parsed = []

    for entry in entries:
        data = {field: "" for field in fields}
        lines = entry.splitlines()

        for line in lines:
            line = line.strip()

            for field in fields:
                if line.startswith(field + ":"):
                    # Extract everything after "Field: "
                    value = line[len(field) + 1:].strip()
                    data[field] = value

        # Only keep entries that actually have content
        if any(data.values()):
            parsed.append(data)

    return parsed

# Read file
with open(input_file, "r", encoding="utf-8") as f:
    content = f.read()

# Parse entries
entries = parse_entries(content)

# Write TSV
with open(output_file, "w", newline="", encoding="utf-8") as f:
    writer = csv.DictWriter(f, fieldnames=fields, delimiter="\t")
    writer.writeheader()
    writer.writerows(entries)

print(f"Done! Parsed {len(entries)} entries into {output_file}")
