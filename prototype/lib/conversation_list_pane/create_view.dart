import 'package:flutter/material.dart';
import 'package:prototype/styles.dart';

class CreateView extends StatefulWidget {
  final String title;
  final String prompt;
  final String hint;
  final String action;

  @override
  const CreateView(
      BuildContext context, this.title, this.prompt, this.hint, this.action,
      {super.key});

  @override
  State<CreateView> createState() => _CreateViewState();
}

class _CreateViewState extends State<CreateView> {
  bool _isInputValid = false;

  final TextEditingController _controller = TextEditingController();

  @override
  Widget build(context) {
    return AlertDialog(
      title: Text(widget.title),
      titlePadding: const EdgeInsets.all(20),
      titleTextStyle: const TextStyle(
        fontFamily: fontFamily,
        fontWeight: FontWeight.bold,
        fontSize: 20,
        color: colorGreyDark,
      ),
      actionsAlignment: MainAxisAlignment.spaceBetween,
      actionsPadding: const EdgeInsets.all(20),
      buttonPadding: const EdgeInsets.symmetric(horizontal: 20, vertical: 20),
      contentPadding: const EdgeInsets.symmetric(horizontal: 20, vertical: 10),
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(10),
      ),
      content: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              const SizedBox(height: 50),
              Text(widget.prompt, style: labelStyle),
              const SizedBox(height: 20),
              Form(
                autovalidateMode: AutovalidateMode.always,
                child: ConstrainedBox(
                  constraints: BoxConstraints.tight(const Size(350, 80)),
                  child: TextFormField(
                    style: inputTextStyle,
                    autofocus: true,
                    controller: _controller,
                    decoration: inputDecoration.copyWith(
                      hintText: widget.hint,
                    ),
                    onChanged: (String value) {
                      setState(() {
                        _isInputValid = value.isNotEmpty;
                      });
                    },
                  ),
                ),
              ),
            ],
          )
        ],
      ),
      actions: <Widget>[
        TextButton(
          style: dynamicTextButtonStyle(context, true, false),
          child: const Text('Cancel'),
          onPressed: () {
            Navigator.of(context).pop('');
          },
        ),
        TextButton(
          style: dynamicTextButtonStyle(context, _isInputValid, true),
          onPressed: _controller.text.isNotEmpty
              ? () {
                  Navigator.of(context).pop(_controller.text);
                }
              : null,
          child: Text(widget.action),
        ),
      ],
    );
  }
}
